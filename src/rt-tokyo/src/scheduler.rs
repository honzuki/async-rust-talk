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

enum ExecStatus {
    Work,
    NoTasks,
    NoPending,
}

impl Scheduler {
    /// Spawn a new task on the current scheduler
    pub fn spawn(task: impl Future<Output = ()> + 'static) {
        let task = Box::pin(task);
        SCHEDULER.with_borrow_mut(|scheduler| {
            let scheduler = scheduler.as_mut().unwrap();
            let id = scheduler.next_id.next();
            scheduler.tasks.insert(id, task);
            scheduler.schedule(id);
        });
    }

    pub(super) fn schedule(&mut self, id: TaskId) {
        self.pending.push(id);
    }

    pub fn block_on(self, main_task: impl Future<Output = ()> + 'static) {
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

        // spawn main task and start scheduling
        Self::spawn(main_task);
        Self::start();

        // remove the injected data from thread
        SCHEDULER.take();
        REACTOR.take();
    }

    fn start() {
        loop {
            let pending = SCHEDULER.with_borrow_mut(|scheduler| {
                std::mem::take(&mut scheduler.as_mut().unwrap().pending)
            });

            for id in pending {
                Self::run_task_by_id(id);
            }

            match Self::status() {
                ExecStatus::Work => continue,
                ExecStatus::NoTasks => return,
                ExecStatus::NoPending => {
                    REACTOR.with_borrow(|reactor| reactor.as_ref().unwrap().block())
                }
            }
        }
    }

    fn run_task_by_id(id: TaskId) {
        let task =
            SCHEDULER.with_borrow_mut(|scheduler| scheduler.as_mut().unwrap().tasks.remove(&id));
        let Some(mut task) = task else {
            return;
        };

        let waker: Waker = id.into();
        match task.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Pending => {
                // we need to add the future back into the map
                SCHEDULER.with_borrow_mut(|scheduler| {
                    let scheduler = scheduler.as_mut().unwrap();
                    scheduler.tasks.insert(id, task);
                });
            }
            Poll::Ready(()) => {} // the future is finished executing, we can drop it
        }
    }

    fn status() -> ExecStatus {
        SCHEDULER.with_borrow(|scheduler| {
            let scheduler = scheduler.as_ref().unwrap();
            if scheduler.tasks.is_empty() {
                return ExecStatus::NoTasks;
            } else if scheduler.pending.is_empty() {
                return ExecStatus::NoPending;
            } else {
                return ExecStatus::Work;
            }
        })
    }
}
