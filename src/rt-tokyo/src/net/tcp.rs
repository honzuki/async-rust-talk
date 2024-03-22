use std::{
    future::Future,
    io::Read,
    pin::Pin,
    task::{Context, Poll},
};

use nix::sys::epoll::EpollFlags;
use pin_project::pin_project;

use crate::scheduler;

pub struct Listener {
    listener: std::net::TcpListener,
}

impl Listener {
    pub fn bind(addr: impl std::net::ToSocketAddrs) -> std::io::Result<Self> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        Ok(Self { listener })
    }

    pub fn accept(&self) -> impl Future<Output = ListenerAcceptOutput> + '_ {
        ListenerAccept { listener: self }
    }
}

struct ListenerAccept<'a> {
    listener: &'a Listener,
}

type ListenerAcceptOutput = std::io::Result<(Stream, super::SocketAddr)>;

impl<'a> Future for ListenerAccept<'a> {
    type Output = ListenerAcceptOutput;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.listener.accept() {
            Ok((stream, addr)) => Poll::Ready(Ok((Stream::new(stream)?, addr))),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                scheduler::SCHEDULER.with_borrow_mut(|scheduler| {
                    scheduler.as_mut().unwrap().reactor().register(
                        &self.listener.listener,
                        cx.waker().clone(),
                        EpollFlags::EPOLLIN,
                    )
                });
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

pub struct Stream {
    stream: std::net::TcpStream,
}

impl Stream {
    fn new(stream: std::net::TcpStream) -> std::io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self { stream })
    }

    pub fn read<'a, 'b>(
        &'a mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = StreamReadOutput> + 'a
    where
        'b: 'a,
    {
        StreamRead { stream: self, buf }
    }
}

#[pin_project]
struct StreamRead<'a, 'b> {
    stream: &'a mut Stream,
    buf: &'b mut [u8],
}

type StreamReadOutput = std::io::Result<usize>;

impl<'a, 'b> Future for StreamRead<'a, 'b> {
    type Output = StreamReadOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let stream = &mut this.stream.stream;
        let buf = this.buf;

        match stream.read(buf) {
            Ok(size) => Poll::Ready(Ok(size)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                scheduler::SCHEDULER.with_borrow_mut(|scheduler| {
                    scheduler.as_mut().unwrap().reactor().register(
                        stream,
                        cx.waker().clone(),
                        EpollFlags::EPOLLIN | EpollFlags::EPOLLOUT,
                    )
                });
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
