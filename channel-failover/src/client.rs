//! If `unwrap`s are replaced with loop break, garbage collection should works fine.
//! Keeping the consumers tasks and the metadata task in a `JoinSet` would abort the tasks faster.
//!
//! Consumer are clonable and could be kept in the client making the implementation of a `commit`
//! method possible (could be called in a `Drop` impl too).
//!
//! Manually storing offset / commiting non-active consumers should be done for messages that are
//! close to the active consumer's ones.

use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    mem,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};
use jiff::{SignedDuration, Timestamp};
use tokio::{task, time};
use tracing::{info, warn};

use crate::{
    consumer,
    consumer::{Consumer, Message},
};

pub const TOPICS_MOCK: [usize; 3] = [7, 42, 99];

pub struct Client {
    topics: HashMap<usize, TopicManager>,
    rx: Receiver<Op>,
    message_queue: VecDeque<Message>,
}

impl Client {
    pub fn new(mock_consumers: usize) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        tokio::spawn(Self::select_source_loop(mock_consumers, tx.clone()));

        for idx in 0..mock_consumers {
            task::spawn({
                let tx = tx.clone();
                async move {
                    let mut consumer = Consumer::new(idx);
                    loop {
                        tx.send(Op::Message(consumer.poll().await)).unwrap();
                    }
                }
            });
        }

        Self {
            topics: TOPICS_MOCK
                .into_iter()
                .map(|topic| (topic, TopicManager::new(mock_consumers, 0)))
                .collect(),
            rx,
            message_queue: VecDeque::new(),
        }
    }

    pub async fn recv(&mut self) -> Message {
        if let Some(msg) = self.message_queue.pop_front() {
            // Rate limit here.
            return msg;
        }

        loop {
            match self.rx.recv().unwrap() {
                Op::Message(msg) => {
                    let topic = self.topics.get_mut(&msg.topic).unwrap();
                    if let Some(msg) = topic.process(msg) {
                        // Rate limit here.
                        return msg;
                    }
                }
                Op::ConsumerSwitch {
                    topic,
                    consumer_idx,
                } => {
                    let topic = self.topics.get_mut(&topic).unwrap();
                    self.message_queue = topic.switch_consumer(consumer_idx);
                    info!(message_count = self.message_queue.len(), "queuing messages");

                    return Box::pin(self.recv()).await;
                }
            }
        }
    }

    async fn select_source_loop(mock_consumers: usize, tx: Sender<Op>) {
        // Fetch watermarks for all topics.
        // If pps is too different, move to best pps.
        // If all consumers have similar pps, switch to best ping if needed.

        // Ping brokers.
        // If difference is too high, switch (all?) topic(s) to best consumer.

        // Re-evaluate topic matcher, subscribe with new topic list.
        // (need to test if calling `.subscribe()` again create interruption on all ready subscribed topics)

        let mut last_distribution = TOPICS_MOCK
            .into_iter()
            .map(|topic| (topic, 0))
            .collect::<HashMap<_, _>>();

        // Mock consumer change.
        loop {
            let pre_watermarks = consumer::watermarks().await;
            time::sleep(Duration::from_secs(3)).await;
            let post_watermarks = consumer::watermarks().await;

            let mut distribution = HashMap::new();
            for topic in TOPICS_MOCK {
                let changes = (0..mock_consumers)
                    .map(|consumer| {
                        pre_watermarks[&consumer][&topic].cmp(&post_watermarks[&consumer][&topic])
                    })
                    .collect::<Vec<_>>();
                if changes.iter().all(|&change| change == Ordering::Equal) {
                    distribution.insert(topic, last_distribution[&topic]);
                } else {
                    distribution.insert(
                        topic,
                        changes
                            .iter()
                            .position(|&change| change == Ordering::Less)
                            .unwrap(),
                    );
                };
            }

            for (&topic, &consumer_idx) in &distribution {
                if last_distribution[&topic] != consumer_idx {
                    warn!(topic, consumer = consumer_idx, "changing topic consumer",);
                    tx.send(Op::ConsumerSwitch {
                        topic,
                        consumer_idx,
                    })
                    .unwrap();
                }
            }
            last_distribution = distribution;

            time::sleep(Duration::from_secs(20)).await;
        }
    }
}

struct TopicManager {
    active_consumer: usize,
    buffers: HashMap<usize, VecDeque<Message>>,
    last_message_reception: Option<Timestamp>,
    message_min_reception: Timestamp,
    // Should be moved to `buffers` by wrapping it in an `Option`.
    buff_messages: bool,
}

impl TopicManager {
    fn new(mock_consumers: usize, active_consumer: usize) -> Self {
        Self {
            active_consumer,
            // Could be replaced with an empty HashMap and use `.entry`.
            buffers: (0..mock_consumers)
                .map(|consumer_idx| (consumer_idx, VecDeque::new()))
                .collect(),
            last_message_reception: None,
            message_min_reception: Timestamp::MIN,
            buff_messages: true,
        }
    }

    fn process(&mut self, msg: Message) -> Option<Message> {
        if msg.reception < self.message_min_reception {
            return None;
        }

        if self.active_consumer == msg.consumer_idx {
            assert!(self.buffers[&self.active_consumer].is_empty());
            self.last_message_reception = Some(msg.reception);
            return Some(msg);
        }
        if !self.buff_messages {
            return None;
        }

        let consumer_buffer = self.buffers.get_mut(&msg.consumer_idx).unwrap();
        Self::clear_expired(
            consumer_buffer,
            msg.reception - SignedDuration::from_secs(30),
        );
        // debug!(
        //     consumer = msg.consumer_idx,
        //     topic = msg.topic,
        //     message_count = consumer_buffer.len() + 1,
        //     "caching message"
        // );
        consumer_buffer.push_back(msg);

        None
    }

    fn switch_consumer(&mut self, consumer_idx: usize) -> VecDeque<Message> {
        self.active_consumer = consumer_idx;
        let consumer_buffer = self.buffers.get_mut(&consumer_idx).unwrap();
        if let Some(last_message_reception) = self.last_message_reception {
            Self::clear_expired(
                consumer_buffer,
                last_message_reception + SignedDuration::from_secs(2),
            );
            self.message_min_reception = last_message_reception + SignedDuration::from_secs(2);
        }
        mem::take(consumer_buffer)
    }

    fn clear_expired(messages: &mut VecDeque<Message>, until: Timestamp) {
        while let Some(msg) = messages.front()
            && msg.reception < until
        {
            assert!(messages.pop_front().is_some());
        }
    }
}

#[derive(Debug)]
enum Op {
    Message(Message),
    ConsumerSwitch { topic: usize, consumer_idx: usize },
}
