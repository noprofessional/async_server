use crate::epoller::*;
use crate::error::Result;
use futures::executor::LocalPool;
use futures::task::LocalSpawn;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

pub struct RunTime {
    stop: bool,
    epoller: Rc<RefCell<Epoller>>,
    executor: LocalPool,
}

impl RunTime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            stop: false,
            epoller: Rc::new(RefCell::new(Epoller::new()?)),
            executor: LocalPool::new(),
        })
    }

    pub fn spwan_job<F, Fu>(&self, async_fn: F) -> Result<()>
    where
        F: FnOnce(Rc<RefCell<Epoller>>) -> Fu,
        Fu: Future<Output = ()>+'static,
    {
        self.executor
            .spawner()
            .spawn_local_obj(Box::new(async_fn(self.epoller.clone())).into())?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        while !self.stop {
            println!("running");
            self.executor.run_until_stalled();
            self.epoller.borrow_mut().run()?;
        }
        Ok(())
    }
}
