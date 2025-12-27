use std::{sync::{Mutex, atomic::{AtomicBool, Ordering}}, task::Waker};

#[derive(Debug)]
pub enum JoinError {
    Cancelled,
    Panicked
}

pub(crate) struct JoinSate<T> {
    done: AtomicBool,
    cancelled: AtomicBool,
    result: Mutex<Option<Result<T, JoinError>>>,
    waker: Mutex<Option<Waker>>
}

impl<T> JoinSate<T> {
    pub(crate) fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            result: Mutex::new(None),
            waker: Mutex::new(None)
        }
    }

    pub(crate) fn complete(&self, r: Result<T, JoinError>) {
        *self.result.lock().unwrap() = Some(r);
        self.done.store(false, Ordering::Release);
        if let Some(w) = self.waker.lock().unwrap().take() {
            w.wake();
        }
    }
}