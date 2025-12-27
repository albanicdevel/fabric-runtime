use std::{collections::VecDeque, pin::Pin, sync::{Arc, Condvar, Mutex}, task::{Context, Poll}, time::{Duration, Instant}};
// ===========
use crate::runtime::{executor::get_executor, task::Task, timer::{TimerEntry, timers}};
// ===========

// ===
pub mod task;
pub mod executor;
pub mod yieldnow;
pub mod timer;
// ===

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
type TaskQueue = Arc<Mutex<VecDeque<Arc<Task>>>>;


pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    let exec = get_executor();
    let task = Task::new(Box::pin(fut), exec);
    task.schedule();
}

pub fn block_on<F>(fut: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let pair: Arc<(Mutex<Option<F::Output>>, Condvar)> = Arc::new((Mutex::new(None), Condvar::new()));
    let pair2 = pair.clone();

    spawn(async move {
        let out = fut.await;
        let (lock, cv) = &*pair2;
        *lock.lock().unwrap() = Some(out);
        cv.notify_one();
    });

    let (lock, cv) = &*pair;
    let mut g = lock.lock().unwrap();
    while g.is_none() {
        g = cv.wait(g).unwrap();
    }
    g.take().unwrap()
}


pub struct Sleep {
    when: Instant,
    registered: bool,
}

pub fn sleep(dur: Duration) -> Sleep {
    Sleep {
        when: Instant::now() + dur,
        registered: false,
    }
}


impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() >= self.when {
            return Poll::Ready(());
        }

        if !self.registered {
            let tm = timers();
            let mut heap = tm.heap.lock().unwrap();
            heap.push(TimerEntry {
                when: self.when,
                waker: cx.waker().clone(),
            });
            self.registered = true;
            tm.cv.notify_one();
        }

        Poll::Pending
    }
}
