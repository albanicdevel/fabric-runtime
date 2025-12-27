use std::time::Duration;

use crate::runtime::{block_on, sleep, spawn, yieldnow::yield_now};

mod runtime;


fn main() {
    block_on(async {
        spawn(async {
            println!("a0");
            yield_now().await;
            println!("b1");
        });

        spawn(async {
            println!("a0");
            yield_now().await;
            println!("b1");
        });

        yield_now().await;

        println!("sleeping");
        sleep(Duration::from_millis(2000)).await;
        println!("woke up");
    });
}
