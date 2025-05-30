use prometheus::{Encoder as _, IntCounterVec, TextEncoder, register_int_counter_vec};
use std::io;
use std::sync::LazyLock;

static METRIC: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!("metric", "description", &["label1", "label2"]).unwrap()
});

fn main() {
    METRIC.with_label_values(&["value1", "value2"]).inc();
    METRIC.with_label_values(&["value3", "value4"]).inc();
    print_metrics();

    METRIC.remove_label_values(&["value1", "value2"]).unwrap();
    print_metrics();
}

fn print_metrics() {
    TextEncoder::new()
        .encode(&prometheus::gather(), &mut io::stdout())
        .unwrap();
}
