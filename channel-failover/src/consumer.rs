use std::{
    collections::HashMap,
    iter::Iterator,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use jiff::Timestamp;
use tokio::{sync::Mutex, time};
use tracing::warn;

use crate::client::TOPICS_MOCK;

static WATERMARKS: LazyLock<Mutex<HashMap<usize, HashMap<usize, usize>>>> = LazyLock::new(|| {
    Mutex::new(
        (0..2)
            .map(|consumer| (consumer, HashMap::from(TOPICS_MOCK.map(|topic| (topic, 0)))))
            .collect(),
    )
});

#[derive(Clone, Debug)]
pub struct Consumer {
    idx: usize,
    message_idx: Arc<AtomicUsize>,
    receiving_topics: Vec<usize>,
}

impl Consumer {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            message_idx: Arc::new(AtomicUsize::new(0)),
            receiving_topics: TOPICS_MOCK.to_vec(),
        }
    }

    pub async fn poll(&mut self) -> Message {
        time::sleep(Duration::from_secs(1)).await;

        // Mock change of receiving topics.
        if rand::random_ratio(1, 10) && self.idx == 0
        // && CAN_CHANGE.fetch_and(false, Ordering::Relaxed)
        {
            warn!(consumer = self.idx, "mocking topic change");
            self.receiving_topics = TOPICS_MOCK.to_vec();
            if rand::random_ratio(TOPICS_MOCK.len() as u32, TOPICS_MOCK.len() as u32 + 1) {
                self.receiving_topics
                    .remove(rand::random_range(0..TOPICS_MOCK.len()));
                warn!(consumer = self.idx, topics = ?self.receiving_topics, "topic list reduced");
            }
        }

        let topic = self.receiving_topics
            [self.message_idx.fetch_add(1, Ordering::Relaxed) % self.receiving_topics.len()];
        let mut watermarks = WATERMARKS.lock().await;
        let watermark = watermarks
            .entry(self.idx)
            .or_default()
            .entry(topic)
            .or_default();
        *watermark += 1;

        Message {
            consumer_idx: self.idx,
            topic,
            idx: *watermark,
            reception: Timestamp::now(),
        }
    }
}

#[derive(Debug)]
pub struct Message {
    pub consumer_idx: usize,
    pub topic: usize,
    pub idx: usize,
    pub reception: Timestamp,
}

pub async fn watermarks() -> HashMap<usize, HashMap<usize, usize>> {
    WATERMARKS.lock().await.clone()
}
