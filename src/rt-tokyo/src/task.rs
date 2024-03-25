use std::{future::Future, pin::Pin};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(super) struct TaskId(usize);

pub type Task = Pin<Box<dyn Future<Output = ()>>>;

impl TaskId {
    pub fn next(&mut self) -> TaskId {
        let next = TaskId(self.0);
        self.0 += 1;
        next
    }

    pub fn to_ptr(self) -> *const () {
        self.0 as _
    }

    pub fn from_ptr(ptr: *const ()) -> Self {
        Self(ptr as _)
    }
}

impl Default for TaskId {
    fn default() -> Self {
        TaskId(0)
    }
}
