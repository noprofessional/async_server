use crate::runtime;
use libc::*;
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::HashMap;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::io::{Error, ErrorKind, Read, Result};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

macro_rules! safe_syscall {
    ( $tty:expr ) => {{
        let res = unsafe { $tty };
        match res {
            -1 => Err(Error::last_os_error()),
            _ => Ok(res),
        }
    }};
}

#[derive(Clone, Copy)]
enum Interest {
    READ,
    WRITE,
}

impl Interest {
    fn to_cint(&self) -> c_int {
        match self {
            Interest::READ => EPOLLIN,
            Interest::WRITE => EPOLLOUT,
        }
    }
}
impl PartialEq for Interest {
    fn eq(&self, other: &Self) -> bool {
        return self.to_cint() == other.to_cint();
    }
}
impl Eq for Interest {}

impl Hash for Interest {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.to_cint().hash(state);
    }
}

pub struct Epoller {
    stop: bool,
    epoll_fd: RawFd,
    event_buf: [epoll_event; 1024],
    interest_map: HashMap<RawFd, c_int>,
    waker_map: HashMap<RawFd, HashMap<Interest, Option<Waker>>>,
    //executor: LocalPool,
}

impl Epoller {
    pub fn new() -> Result<Self> {
        let epoll_fd = safe_syscall!(epoll_create1(0))?;
        Ok(Epoller {
            stop: false,
            epoll_fd: epoll_fd,
            event_buf: [epoll_event { events: 0, u64: 0 }; 1024],
            interest_map: HashMap::new(),
            waker_map: HashMap::new(),
            //executor: LocalPool::new(),
        })
    }

    fn register(&mut self, handle: RawFd, interest: Interest, waker: Waker) -> Result<()> {
        let interest_int = interest.to_cint();
        let val = self.interest_map.get(&handle);
        match val {
            Some(old) => {
                if (old & interest_int) == 0 {
                    // modify to system
                    let new = old | interest_int;

                    println!("new interest");
                    let mut ev = epoll_event {
                        events: new as u32,
                        u64: handle as u64,
                    };

                    safe_syscall!(epoll_ctl(
                        self.epoll_fd,
                        EPOLL_CTL_MOD,
                        handle,
                        &mut ev as *mut epoll_event
                    ))?;
                    self.interest_map.insert(handle, interest_int);
                } else {
                    println!("same interest");
                }
            }
            None => {
                println!("new handle");
                // create new
                let mut ev = epoll_event {
                    events: interest_int as u32,
                    u64: handle as u64,
                };
                safe_syscall!(epoll_ctl(
                    self.epoll_fd,
                    EPOLL_CTL_ADD,
                    handle,
                    &mut ev as *mut epoll_event
                ))?;
                self.interest_map.insert(handle, interest_int);
            }
        }

        //add waker map
        match self.waker_map.entry(handle) {
            HashMapEntry::Occupied(mut entry) => {
                entry.get_mut().insert(interest, Some(waker));
                entry.get_mut()
            }
            HashMapEntry::Vacant(entry) => entry.insert({
                let mut res = HashMap::new();
                res.insert(interest, Some(waker));
                res
            }),
        };
        Ok(())
    }

    // borrow mutable
    pub fn run(&mut self) -> Result<()> {
        let cnt = safe_syscall!(epoll_wait(
            self.epoll_fd,
            &mut self.event_buf as *mut epoll_event,
            1024,
            -1,
        ))?;

        println!("ready {}", cnt);
        for i in 0..cnt as usize {
            let rawfd = self.event_buf[i].u64 as RawFd;
            let happens = self.event_buf[i].events;

            match self.waker_map.entry(rawfd) {
                HashMapEntry::Occupied(mut entry) => {
                    for (interest, maybe_waker) in entry.get_mut() {
                        if (happens & interest.to_cint() as u32) != 0 {
                            if let Some(waker) = maybe_waker.take() {
                                waker.wake();
                            };
                        }
                    }
                }
                HashMapEntry::Vacant(_) => {
                    println!(
                        "raw fd {} not exist waker, event:{} ignored.",
                        rawfd, happens
                    );
                }
            }
        }
        Ok(())
    }
}

pub struct AsyncTcpListener {
    listener: TcpListener,
}

impl AsyncTcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener: listener })
    }

    pub fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        self.listener.accept()
    }

    pub fn async_accept(&self) -> AcceptFuture {
        AcceptFuture { listener: self }
    }
}

impl AsRawFd for AsyncTcpListener {
    fn as_raw_fd(&self) -> RawFd {
        return self.listener.as_raw_fd();
    }
}

pub struct AcceptFuture<'a> {
    listener: &'a AsyncTcpListener,
}

impl Future for AcceptFuture<'_> {
    type Output = Result<AsyncTcpStream>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                println!("new conn from:{}", addr);
                return Poll::Ready(AsyncTcpStream::from_std(stream));
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // garanteed to has epoller
                println!("need wait");
                let mut_self = self.get_mut();
                match runtime::epoller().borrow_mut().register(
                    mut_self.listener.as_raw_fd(),
                    Interest::READ,
                    cx.waker().clone(),
                ) {
                    Ok(_) => return Poll::Pending,
                    Err(err) => return Poll::Ready(Err(err)),
                }
            }
            Err(err) => {
                println!("err occur");
                return Poll::Ready(Err(err));
            }
        }
    }
}

struct Buffer {
    data: [u8; 1024],
}
#[derive(Debug)]
pub struct AsyncTcpStream {
    stream: TcpStream,
}

impl AsyncTcpStream {
    fn from_std(stream: TcpStream) -> Result<Self> {
        stream
            .set_nonblocking(true)
            .and_then(|_| Ok(Self { stream: stream }))
    }

    pub fn async_read<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> TcpReadFutrue<'a, 'b> {
        return TcpReadFutrue {
            stream: self,
            buf: buf,
        };
    }

    fn read<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> Result<usize> {
        return self.stream.read(buf);
    }
}

impl AsRawFd for AsyncTcpStream {
    fn as_raw_fd(&self) -> RawFd {
        return self.stream.as_raw_fd();
    }
}

pub struct TcpReadFutrue<'a, 'b> {
    stream: &'a mut AsyncTcpStream,
    buf: &'b mut [u8],
}

impl<'a, 'b> Future for TcpReadFutrue<'a, 'b> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut_self = self.get_mut();
        match mut_self.stream.read(mut_self.buf) {
            Ok(cnt) => {
                println!("data size:{}", cnt);
                return Poll::Ready(Ok(cnt));
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // garanteed to has epoller
                println!("need wait data");
                match runtime::epoller().borrow_mut().register(
                    mut_self.stream.as_raw_fd(),
                    Interest::READ,
                    cx.waker().clone(),
                ) {
                    Ok(_) => return Poll::Pending,
                    Err(err) => {
                        println!("register stream err:{}", err);
                        return Poll::Ready(Err(err));
                    }
                }
            }
            Err(err) => {
                println!("read tcp stream err:{}", err);
                return Poll::Ready(Err(err));
            }
        }
    }
}
