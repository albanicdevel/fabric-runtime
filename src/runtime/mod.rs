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


/// Registers a new asynchronous task in the runtime.
///
/// This function **does NOT create a new OS thread**.
/// Instead, it:
/// - wraps the provided `Future` into a `Task`,
/// - registers the task in the executor queue,
/// - and notifies worker threads that new work is available.
///
/// The task will be executed by existing worker threads.
///
/// ## Important
/// Calling `spawn` alone does not keep the program alive.
/// Without [`block_on`] (or another mechanism that waits),
/// the process may exit before the task is executed.
///
/// ## Examples
///
/// Registering a task (may not run without `block_on`):
/// ```
/// spawn(async {
///     println!("hello!");
/// });
/// ```
///
/// Correct usage with `block_on`:
/// ```
/// block_on(async {
///     spawn(async {
///         println!("R: 1");
///         println!("R: 2");
///
///         yield_now().await;
///
///         println!("R: 1-1");
///         println!("R: 2-2");
///     });
///
///     // Give spawned tasks a chance to run
///     yield_now().await;
/// });
/// ```
/// - `spawn` does not create threads
/// - `spawn` registers a task
/// - `schedule` puts the task into the executor queue
/// - worker threads poll tasks
///
pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    let exec = get_executor();
    let task = Task::new(Box::pin(fut), exec);
    task.schedule();
}

/// sync code ── block_on ──▶ async world
/// 
///                 ▲
///                 │
///               wait
/// imagine that block_on is a bridge between the sync and async world
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

/// Creates a future that completes after the specified duration.
///
/// This function does **not block the current thread**.
/// Instead, it returns a `Sleep` future that:
/// - yields `Poll::Pending` until the deadline is reached,
/// - registers the current task's `Waker` with the global timer manager,
/// - and is woken up by a timer thread once the duration has elapsed.
///
/// Internally, the task is suspended and rescheduled by the executor
/// when the timer fires.
///
/// ## Example
/// ```
/// block_on(async {
///     println!("sleeping...");
///     sleep(Duration::from_secs(1)).await;
///     println!("woke up");
/// });
/// ```
///
/// - `sleep` does **not** create a new thread.
/// - Multiple sleeping tasks are managed by a shared timer thread.
/// - The returned future completes with `()` when the timeout expires.
///
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
