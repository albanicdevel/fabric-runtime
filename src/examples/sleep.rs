use runtime::{block_on, sleep};
use std::time::Duration;

fn main() {
    block_on(async {
        println!("sleeping...");
        sleep(Duration::from_millis(500)).await;
        println!("woke up");
    });
}
