use std::task::Poll;

pub struct YieldNow {
    yielded: bool
}

pub fn yield_now() -> YieldNow { 
    YieldNow { yielded: false }
}

impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if !self.yielded {
            self.yielded = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(())
    }
}