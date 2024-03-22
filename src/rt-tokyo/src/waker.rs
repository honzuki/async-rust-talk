use std::{
    mem::ManuallyDrop,
    rc::Rc,
    task::{RawWaker, RawWakerVTable, Waker},
};

use crate::{scheduler::SCHEDULER, task::TaskId};

struct Data {
    id: TaskId,
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
    let data = std::mem::ManuallyDrop::new(Rc::from_raw(ptr as *const Data));
    let _: ManuallyDrop<_> = data.clone();

    RawWaker::new(ptr, &VTABLE)
}

unsafe fn wake(ptr: *const ()) {
    wake_by_ref(ptr);
    drop_waker(ptr);
}

unsafe fn wake_by_ref(ptr: *const ()) {
    let data = std::mem::ManuallyDrop::new(Rc::from_raw(ptr as *const Data));
    SCHEDULER.with_borrow_mut(|scheduler| scheduler.as_mut().unwrap().schedule(data.id));
}

unsafe fn drop_waker(ptr: *const ()) {
    drop(Rc::from_raw(ptr as *const Data));
}

impl From<TaskId> for Waker {
    fn from(id: TaskId) -> Self {
        let waker = Rc::new(Data { id });
        unsafe { Self::from_raw(RawWaker::new(Rc::into_raw(waker) as _, &VTABLE)) }
    }
}
