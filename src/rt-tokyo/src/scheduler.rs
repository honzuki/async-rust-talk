use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    task::{Context, Poll, Waker},
};

use crate::{
    reactor::Reactor,
    task::{Task, TaskId},
};

thread_local! {
    pub(super) static SCHEDULER: RefCell<Option<Scheduler>> = RefCell::new(None);
    pub(super) static REACTOR: RefCell<Option<Reactor>> = RefCell::new(None);
}

#[derive(Default)]
pub struct Scheduler {
    next_id: TaskId,
    tasks: HashMap<TaskId, Task>,
    pending: Vec<TaskId>,
}

impl Scheduler {
    /// Spawn a new task on the current scheduler
    pub fn spawn(task: impl Future<Output = ()> + 'static) {
        let task = Box::pin(task);
        SCHEDULER.with_borrow_mut(|scheduler| {
            let scheduler = scheduler.as_mut().unwrap();
            let id = scheduler.next_id.next();
            scheduler.tasks.insert(id, task);
            scheduler.pending.push(id);
        });
    }

    pub(super) fn schedule(&mut self, id: TaskId) {
        self.pending.push(id);
    }

    pub fn block_on<T, O>(self, main_task: T) -> O
    where
        T: Future<Output = O>,
    {
        // inject necessary data into the thread
        SCHEDULER.with_borrow_mut(|scheduler| {
            if scheduler.is_some() {
                panic!("can not spawn more than 1 run time on the same thread");
            }

            *scheduler = Some(self);
        });
        REACTOR.with_borrow_mut(|reactor| {
            *reactor = Some(Reactor::default());
        });

        // execute main future
        let output = Self::execute(main_task);

        // remove the injected data from thread
        SCHEDULER.take();
        REACTOR.take();

        output
    }

    fn execute<T, O>(main_task: T) -> O
    where
        T: Future<Output = O>,
    {
        let mut main_task = std::pin::pin!(main_task);
        let main_id = SCHEDULER.with_borrow_mut(|scheduler| {
            let scheduler = scheduler.as_mut().unwrap();
            let id = scheduler.next_id.next();
            scheduler.pending.push(id);
            id
        });
        let main_waker: Waker = main_id.into();

        loop {
            let pending = SCHEDULER.with_borrow_mut(|scheduler| {
                std::mem::take(&mut scheduler.as_mut().unwrap().pending)
            });
            for id in pending {
                if id == main_id {
                    if let Poll::Ready(output) = main_task
                        .as_mut()
                        .poll(&mut Context::from_waker(&main_waker))
                    {
                        return output;
                    }

                    continue;
                }

                let task = SCHEDULER
                    .with_borrow_mut(|scheduler| scheduler.as_mut().unwrap().tasks.remove(&id));
                let Some(mut task) = task else {
                    continue;
                };

                let waker: Waker = id.into();
                if matches!(
                    task.as_mut().poll(&mut Context::from_waker(&waker)),
                    Poll::Pending
                ) {
                    SCHEDULER.with_borrow_mut(|scheduler| {
                        let scheduler = scheduler.as_mut().unwrap();
                        scheduler.tasks.insert(id, task);
                    });
                }
            }

            if SCHEDULER.with_borrow(|scheduler| {
                let scheduler = scheduler.as_ref().unwrap();
                scheduler.pending.is_empty()
            }) {
                // block on reactor as we truly do not have any work left
                REACTOR.with_borrow(|reactor| reactor.as_ref().unwrap().block())
            }
        }
    }
}
