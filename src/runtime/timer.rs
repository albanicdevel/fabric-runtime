use std::{cmp::Ordering, collections::BinaryHeap, sync::{Condvar, Mutex, OnceLock}, task::Waker, time::Instant};

pub struct TimerEntry {
    pub when: Instant,
    pub waker: Waker
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.when.cmp(&self.when)
    }
}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}
impl Eq for TimerEntry {}

pub struct TimerManager {
    pub(crate) heap: Mutex<BinaryHeap<TimerEntry>>,
    pub(crate) cv: Condvar,
}

static TIMERS: OnceLock<TimerManager> = OnceLock::new();

pub fn timers() -> &'static TimerManager {
    TIMERS.get_or_init(|| {
        let tm = TimerManager {
            heap: Mutex::new(BinaryHeap::new()),
            cv: Condvar::new(),
        };

        std::thread::spawn(timer_thread);
        tm
    })
}

pub fn timer_thread() {
    let tm = timers();

    loop {
        let mut heap = tm.heap.lock().unwrap();

        while let Some(entry) = heap.peek() {
            let now = Instant::now();

            if entry.when <= now {
                let entry = heap.pop().unwrap();
                drop(heap);
                entry.waker.wake();
                heap = tm.heap.lock().unwrap();
            } else {
                let timeout = entry.when - now;
                let (h, _) = tm.cv.wait_timeout(heap, timeout).unwrap();
                heap = h;
            }
        }

        heap = tm.cv.wait(heap).unwrap();
    }
}
