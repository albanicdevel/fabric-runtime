use std::{sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, task::{Context, Poll, RawWaker, RawWakerVTable, Waker}};

use crate::runtime::{BoxFuture, TaskQueue, executor::Executor};

pub struct Task {
    future: Mutex<Option<BoxFuture>>,
    queue: TaskQueue,
    scheduled: AtomicBool,
    exec: &'static Executor,
}

impl Task {
    pub fn new(fut: BoxFuture, exec: &'static Executor) -> Arc<Self> {
        Arc::new(Self {
            future: Mutex::new(Some(fut)),
            queue: exec.queue.clone(),
            scheduled: AtomicBool::new(false),
            exec,
        })
    }

    pub fn schedule(self: &Arc<Self>) {
        // Ensure we don't enqueue the same task multiple times concurrently.
        if !self.scheduled.swap(true, Ordering::AcqRel) {
            {
                let mut q = self.queue.lock().unwrap();
                q.push_back(self.clone());
            }
            self.exec.cv.notify_one();
        }
    }

    pub fn poll(self: &Arc<Self>) {
        // Mark as not scheduled; if it becomes pending and needs to run again,
        // the waker will reschedule it.
        self.scheduled.store(false, Ordering::Release);

        let waker = unsafe { Waker::from_raw(Self::into_raw_waker(self)) };
        let mut cx = Context::from_waker(&waker);

        // Take the future out to poll without holding the mutex.
        let mut fut = {
            let mut slot = self.future.lock().unwrap();
            match slot.take() {
                Some(f) => f,
                None => return, // already completed
            }
        };

        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(()) => {
                // done
            }
            Poll::Pending => {
                let mut slot = self.future.lock().unwrap();
                *slot = Some(fut);
            }
        }
    }

    fn into_raw_waker(task: &Arc<Task>) -> RawWaker {
        let data = Arc::into_raw(task.clone()) as *const ();
        RawWaker::new(data, &VTABLE)
    }
}

static VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_waker, wake_waker, wake_by_ref_waker, drop_waker);

unsafe fn clone_waker(data: *const ()) -> RawWaker {
    let task = Arc::<Task>::from_raw(data as *const Task);
    let cloned = task.clone();
    // Put the original back (avoid decrementing refcount on drop)
    let _ = Arc::into_raw(task);
    RawWaker::new(Arc::into_raw(cloned) as *const (), &VTABLE)
}

unsafe fn wake_waker(data: *const ()) {
    // wake consumes the waker => we must drop one Arc ref at end (normal drop).
    let task = Arc::<Task>::from_raw(data as *const Task);
    task.schedule();
    // `task` dropped here => refcount -1 (correct for wake)
}

unsafe fn wake_by_ref_waker(data: *const ()) {
    // wake_by_ref must NOT consume the waker reference.
    // Reconstruct Arc but avoid dropping it by turning it back into raw.
    let task = Arc::<Task>::from_raw(data as *const Task);
    task.schedule();
    let _ = Arc::into_raw(task); // keep refcount unchanged
}

unsafe fn drop_waker(data: *const ()) {
    // dropping the waker consumes one Arc strong ref
    let _ = Arc::<Task>::from_raw(data as *const Task);
}