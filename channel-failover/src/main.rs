// #![allow(dead_code, unused_imports)]

use tracing::info;

use crate::client::Client;

pub mod client;
pub mod consumer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("starting");
    let mut client = Client::new(2);
    loop {
        let msg = client.recv().await;
        info!(
            consumer = msg.consumer_idx,
            topic = msg.topic,
            idx = msg.idx,
            reception = %msg.reception,
            "message received"
        );
    }
}
