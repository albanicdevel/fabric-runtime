use runtime::spawn;

fn main() {
    spawn(async {
        println!("hello from task");
    });
}

// This example demonstrates why `block_on` is required:
// without it, the program may exit before async tasks run.
