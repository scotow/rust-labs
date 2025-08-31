use prometheus::Registry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use metrics_registry::Database;

mod metrics;

#[tokio::main]
async fn main() {
    metrics::init();
    tokio::spawn(metrics::serve());

    let database_1 = Database::new();

    let registry_1 = Arc::new(
        Registry::new_custom(
            None,
            Some(HashMap::from_iter([(
                "registry".to_owned(),
                "registry_1".to_owned(),
            )])),
        )
        .unwrap(),
    );
    assert_eq!(metrics::add_registry(Arc::downgrade(&registry_1)), 1);
    let database_2 = Database::with_registry(Arc::clone(&registry_1));
    drop(registry_1);

    time::sleep(Duration::from_secs(5)).await;
    drop(database_2);

    time::sleep(Duration::MAX).await;
}
