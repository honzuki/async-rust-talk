use std::{
    collections::HashMap,
    os::fd::{AsFd, AsRawFd},
    task::Waker,
};

use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};

pub(super) struct Reactor {
    epoll: Epoll,
    events: HashMap<u64, Waker>,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            epoll: Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC).unwrap(),
            events: Default::default(),
        }
    }

    pub fn update_waker<Fd>(&mut self, fd: &Fd, waker: Waker)
    where
        Fd: AsFd + AsRawFd,
    {
        let id = fd.as_raw_fd() as u64;
        self.events.insert(id, waker);
    }

    pub fn register<Fd>(&mut self, fd: &Fd, flags: EpollFlags)
    where
        Fd: AsFd + AsRawFd,
    {
        let id = fd.as_raw_fd() as u64;
        let _ = self.epoll.add(fd, EpollEvent::new(flags, id));
    }

    pub fn remove<Fd>(&mut self, fd: &Fd)
    where
        Fd: AsFd + AsRawFd,
    {
        let id = fd.as_raw_fd() as u64;
        self.events.remove(&id);
        let _ = self.epoll.delete(fd);
    }

    pub fn block(&self) {
        let mut events = [EpollEvent::empty(); 1024];
        let count = self.epoll.wait(&mut events, EpollTimeout::NONE).unwrap();

        for event in &events[..count] {
            let id = event.data();
            if let Some(waker) = self.events.get(&id) {
                waker.wake_by_ref();
            }
        }
    }
}

impl Default for Reactor {
    fn default() -> Self {
        Self::new()
    }
}
