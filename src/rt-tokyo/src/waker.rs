use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::{scheduler::SCHEDULER, task::TaskId};

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
    RawWaker::new(ptr, &VTABLE)
}

unsafe fn wake(ptr: *const ()) {
    wake_by_ref(ptr);
}

unsafe fn wake_by_ref(ptr: *const ()) {
    let id = TaskId::from_ptr(ptr);
    SCHEDULER.with_borrow_mut(|scheduler| scheduler.as_mut().unwrap().schedule(id));
}

unsafe fn drop_waker(_ptr: *const ()) {}

impl From<TaskId> for Waker {
    fn from(id: TaskId) -> Self {
        unsafe { Self::from_raw(RawWaker::new(id.to_ptr(), &VTABLE)) }
    }
}
