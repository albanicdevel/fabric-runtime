use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex, OnceLock}, thread};

use crate::runtime::TaskQueue;

pub struct Executor {
    pub(crate) queue: TaskQueue,
    pub(crate) cv: Condvar,
}

static EXEC: OnceLock<Executor> = OnceLock::new();

pub fn get_executor() -> &'static Executor {
    EXEC.get_or_init(|| {
        let exec = Executor {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            cv: Condvar::new(),
        };

        // Fixed-size thread pool
        let workers = 2usize;
        for _ in 0..workers {
            // SAFETY: `exec` will be moved into OnceLock at the end of this closure.
            // We must not create a fake 'static reference before that.
            //
            // Instead, spawn threads AFTER init by using OnceLock again:
            // We create threads that call `worker_loop(get_executor())` which is 'static.
            thread::spawn(|| worker_loop(get_executor()));
        }

        exec
    })
}

fn worker_loop(exec: &'static Executor) {
    loop {
        let task = {
            let mut q = exec.queue.lock().unwrap();
            while q.is_empty() {
                q = exec.cv.wait(q).unwrap();
            }
            q.pop_front()
        };

        if let Some(task) = task {
            task.poll();
        }
    }
}
