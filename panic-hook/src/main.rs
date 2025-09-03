use std::{panic, thread};
use std::time::Duration;

fn main() {
    panic::set_hook(Box::new(|_| {
        println!("initial panic hook");
    }));

    thread::spawn(|| {
        panic!();
    });

    thread::sleep(Duration::from_secs(1));

    thread::spawn(|| {
        panic::set_hook(Box::new(|_| {
            println!("overwritten panic hook");
        }));
        panic!();
    });

    thread::sleep(Duration::from_secs(1));
    panic!();
}
