use runtime::{block_on, spawn, yield_now};

fn main() {
    block_on(async {
        for i in 0..3 {
            spawn(async move {
                println!("task {i}: start");
                yield_now().await;
                println!("task {i}: end");
            });
        }

        yield_now().await;
    });
}
