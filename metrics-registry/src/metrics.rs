use axum::response::IntoResponse;
use axum::{Router, routing};
use itertools::Itertools;
use prometheus::{Encoder, Registry, TextEncoder, register_int_gauge_vec};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, Weak};
use tokio::net::TcpListener;

static REGISTRIES: LazyLock<Mutex<HashMap<usize, Weak<Registry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static REGISTRY_IDX: AtomicUsize = AtomicUsize::new(1);

pub fn init() {
    register_int_gauge_vec!("global_metric", "global metric", &["label_1", "label_2"])
        .unwrap()
        .with_label_values(&["label_1", "label_2"])
        .set(1);
}

pub async fn serve() {
    axum::serve(
        TcpListener::bind((Ipv4Addr::LOCALHOST, 8080))
            .await
            .unwrap(),
        Router::new().route("/metrics", routing::get(metrics)),
    )
    .await
    .unwrap();
}

async fn metrics() -> impl IntoResponse {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    encoder
        .encode(&prometheus::default_registry().gather(), &mut buffer)
        .unwrap();
    let mut guard = REGISTRIES.lock().unwrap();
    let (registries, to_clear) = guard
        .iter()
        .map(|(&idx, registry)| registry.upgrade().ok_or(idx))
        .partition_result::<Vec<_>, Vec<_>, _, _>();
    for idx in to_clear {
        dbg!(idx);
        assert!(guard.remove(&idx).is_some());
    }
    drop(guard);

    for registry in registries {
        encoder.encode(&registry.gather(), &mut buffer).unwrap();
    }

    buffer
}

pub fn add_registry(registry: Weak<Registry>) -> usize {
    let idx = REGISTRY_IDX.fetch_add(1, Ordering::Relaxed);
    assert_ne!(idx, usize::MAX);
    REGISTRIES.lock().unwrap().insert(idx, registry);
    idx
}
