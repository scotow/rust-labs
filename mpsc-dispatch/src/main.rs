use std::thread;
use std::time::Duration;
use crossbeam_channel::bounded;

fn main() {
    let (tx, rx) = bounded(0);
    thread::scope(|s| {
        for i in 0..10 {
            let rx = rx.clone();
            s.spawn(move || {
                while let Ok(()) = rx.recv() {
                    println!("{i}");
                    if i % 2 == 0 {
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            });
        }
        for _ in 0..100_000 {
            tx.send(()).unwrap();
        }
        drop(tx);
    });
}
