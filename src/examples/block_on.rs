use runtime::{spawn, block_on, yield_now};

fn main() {
    block_on(async {
        spawn(async {
            println!("task started");
            yield_now().await;
            println!("task finished");
        });
        yield_now().await;
    });
}

