use std::sync::Arc;
use prometheus::{IntCounter, Registry};

pub struct Database {
    registry: Arc<Registry>,
    metrics: Metrics,
}

impl Database {
    pub fn new() -> Self {
        Self::with_registry(Arc::new(prometheus::default_registry().clone()))
    }

    pub fn with_registry(registry: Arc<Registry>) -> Self {
        let metrics = Metrics {
            counter_1: metrics::counter_1(&registry, &["label_1", "label_2"]),
        };
        Self {
            registry,
            metrics,
        }
    }
}

struct Metrics {
    counter_1: IntCounter,
}

mod metrics {
    use prometheus::{
        IntCounter, Registry,
        register_int_counter_vec_with_registry,
    };

    pub(super) fn counter_1(registry: &Registry, labels: &[&str]) -> IntCounter {
        register_int_counter_vec_with_registry!(
            "counter_1",
            "help message for counter_1",
            &["label_1", "label_2"],
            registry
        )
        .expect("invalid metrics")
        .with_label_values(labels)
    }
}
