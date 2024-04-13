use std::{
    future::Future,
    io::{Read, Write},
    pin::Pin,
    task::{Context, Poll},
};

use nix::sys::epoll::EpollFlags;
use pin_project::{pin_project, pinned_drop};

use crate::scheduler::REACTOR;

pub struct Listener {
    listener: std::net::TcpListener,
}

impl Listener {
    pub fn bind(addr: impl std::net::ToSocketAddrs) -> std::io::Result<Self> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        Ok(Self { listener })
    }

    pub fn accept(&mut self) -> impl Future<Output = ListenerAcceptOutput> + '_ {
        ListenerAccept::new(self)
    }

    pub fn local_addr(&self) -> std::io::Result<super::SocketAddr> {
        self.listener.local_addr()
    }
}

struct ListenerAccept<'a> {
    listener: &'a mut Listener,
}

impl<'a> ListenerAccept<'a> {
    fn new(listener: &'a mut Listener) -> Self {
        listener.listener.set_nonblocking(true).unwrap();
        REACTOR.with_borrow_mut(|reactor| {
            reactor
                .as_mut()
                .unwrap()
                .register(&listener.listener, EpollFlags::EPOLLIN)
        });
        Self { listener }
    }
}

pub type ListenerAcceptOutput = std::io::Result<(Stream, super::SocketAddr)>;

impl<'a> Future for ListenerAccept<'a> {
    type Output = ListenerAcceptOutput;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.listener.accept() {
            Ok((stream, addr)) => Poll::Ready(Ok((Stream::new(stream)?, addr))),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                REACTOR.with_borrow_mut(|reactor| {
                    reactor
                        .as_mut()
                        .unwrap()
                        .update_waker(&self.listener.listener, cx.waker().clone())
                });
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl<'a> Drop for ListenerAccept<'a> {
    fn drop(&mut self) {
        REACTOR
            .with_borrow_mut(|reactor| reactor.as_mut().unwrap().remove(&self.listener.listener));
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
        StreamRead::new(self, buf)
    }

    pub fn write<'a, 'b>(
        &'a mut self,
        buf: &'b [u8],
    ) -> impl Future<Output = StreamWriteOutput> + 'a
    where
        'b: 'a,
    {
        StreamWrite::new(self, buf)
    }
}

#[pin_project(PinnedDrop)]
struct StreamRead<'a, 'b> {
    stream: &'a mut Stream,
    buf: &'b mut [u8],
}

impl<'a, 'b> StreamRead<'a, 'b> {
    fn new(stream: &'a mut Stream, buf: &'b mut [u8]) -> Self {
        stream.stream.set_nonblocking(true).unwrap();
        REACTOR.with_borrow_mut(|reactor| {
            reactor
                .as_mut()
                .unwrap()
                .register(&stream.stream, EpollFlags::EPOLLIN)
        });

        StreamRead { stream, buf }
    }
}

pub type StreamReadOutput = std::io::Result<usize>;

impl<'a, 'b> Future for StreamRead<'a, 'b> {
    type Output = StreamReadOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let stream = &mut this.stream.stream;
        let buf = this.buf;

        match stream.read(buf) {
            Ok(size) => Poll::Ready(Ok(size)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                REACTOR.with_borrow_mut(|reactor| {
                    reactor
                        .as_mut()
                        .unwrap()
                        .update_waker(stream, cx.waker().clone())
                });

                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

#[pinned_drop]
impl<'a, 'b> PinnedDrop for StreamRead<'a, 'b> {
    fn drop(self: Pin<&mut Self>) {
        REACTOR.with_borrow_mut(|reactor| reactor.as_mut().unwrap().remove(&self.stream.stream));
    }
}

#[pin_project(PinnedDrop)]
struct StreamWrite<'a, 'b> {
    stream: &'a mut Stream,
    buf: &'b [u8],
}

impl<'a, 'b> StreamWrite<'a, 'b> {
    fn new(stream: &'a mut Stream, buf: &'b [u8]) -> Self {
        stream.stream.set_nonblocking(true).unwrap();
        REACTOR.with_borrow_mut(|reactor| {
            reactor
                .as_mut()
                .unwrap()
                .register(&stream.stream, EpollFlags::EPOLLOUT)
        });

        StreamWrite { stream, buf }
    }
}

pub type StreamWriteOutput = std::io::Result<usize>;

impl<'a, 'b> Future for StreamWrite<'a, 'b> {
    type Output = StreamWriteOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let stream = &mut this.stream.stream;
        let buf = this.buf;

        match stream.write(buf) {
            Ok(size) => Poll::Ready(Ok(size)),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                REACTOR.with_borrow_mut(|reactor| {
                    reactor
                        .as_mut()
                        .unwrap()
                        .update_waker(stream, cx.waker().clone())
                });

                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

#[pinned_drop]
impl<'a, 'b> PinnedDrop for StreamWrite<'a, 'b> {
    fn drop(self: Pin<&mut Self>) {
        REACTOR.with_borrow_mut(|reactor| reactor.as_mut().unwrap().remove(&self.stream.stream));
    }
}
