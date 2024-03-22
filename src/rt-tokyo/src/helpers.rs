use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[pin_project]
pub struct Select<F1, F2> {
    #[pin]
    future1: F1,
    #[pin]
    future2: F2,
}

impl<F1, F2> Select<F1, F2> {
    pub fn new(future1: F1, future2: F2) -> Self {
        Self { future1, future2 }
    }
}

pub enum SelectResult<O1, O2> {
    First(O1),
    Second(O2),
}

impl<F1, F2> Future for Select<F1, F2>
where
    F1: Future,
    F2: Future,
{
    type Output = SelectResult<<F1 as Future>::Output, <F2 as Future>::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if let Poll::Ready(output) = this.future1.poll(cx) {
            return Poll::Ready(SelectResult::First(output));
        }

        if let Poll::Ready(output) = this.future2.poll(cx) {
            return Poll::Ready(SelectResult::Second(output));
        }

        return Poll::Pending;
    }
}
