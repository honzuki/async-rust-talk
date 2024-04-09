pub mod helpers;
pub mod net;
mod reactor;
mod scheduler;
mod task;
mod waker;

pub use scheduler::Scheduler;

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    use crate::{
        helpers::{Select, SelectResult},
        Scheduler,
    };

    #[test]
    fn dumb_futures() {
        let scheduler = Scheduler::default();
        assert_eq!(scheduler.block_on(DumbFuture { counter: 10 }), 0);

        let scheduler = Scheduler::default();
        scheduler.block_on(dumb_future_2());
    }

    struct DumbFuture {
        counter: u64,
    }

    impl Future for DumbFuture {
        type Output = u64;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            println!("counter: {}", self.counter);

            if self.counter > 0 {
                self.counter -= 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            Poll::Ready(self.counter)
        }
    }

    async fn dumb_future_2() {
        let d1 = DumbFuture { counter: 20 };
        let d2 = DumbFuture { counter: 10 };

        assert!(matches!(Select::new(d1, d2).await, SelectResult::Second(_)));
    }
}
