use std::{future::Future, pin::Pin};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(super) struct TaskId(u64);

pub type Task = Pin<Box<dyn Future<Output = ()>>>;

impl TaskId {
    pub fn next(&mut self) -> TaskId {
        let next = TaskId(self.0);
        self.0 += 1;
        next
    }
}

impl Default for TaskId {
    fn default() -> Self {
        TaskId(0)
    }
}
