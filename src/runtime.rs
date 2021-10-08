use crate::epoller::*;
use crate::error::Result;
use futures::executor::LocalPool;
use futures::task::{LocalFutureObj, LocalSpawn};
use std::cell::RefCell;
use std::collections::LinkedList;
use std::future::Future;
use std::rc::Rc;

thread_local! {
    static EPOLLER: Rc<RefCell<Epoller>> = Rc::new(RefCell::new(Epoller::new().unwrap()));
}

pub fn epoller() -> Rc<RefCell<Epoller>> {
    EPOLLER.with(|epoller| epoller.clone())
}

thread_local! {
    static EXECUTOR: Rc<RefCell<LocalPool>> = Rc::new(RefCell::new(LocalPool::new()));
}

pub fn executor() -> Rc<RefCell<LocalPool>> {
    EXECUTOR.with(|executor| executor.clone())
}

thread_local! {
    static CACHE_LOCAL_OBJ:Rc<RefCell<LinkedList<LocalFutureObj<'static, ()>>>>
        = Rc::new(RefCell::new(LinkedList::new()));
}

fn cache() -> Rc<RefCell<LinkedList<LocalFutureObj<'static, ()>>>> {
    CACHE_LOCAL_OBJ.with(|cache| cache.clone())
}

pub fn spawn_task(future: impl Future<Output = ()> + 'static) -> Result<()> {
    match executor().try_borrow() {
        Ok(executor) => {
            executor
                .spawner()
                .spawn_local_obj(Box::new(future).into())?;
        }
        // means spawn happens while running executor
        // should push into cache and spawn after this run
        Err(_) => cache().borrow_mut().push_back(Box::new(future).into()),
    };
    Ok(())
}

pub fn run() -> Result<()> {
    loop {
        println!("running");
        executor().borrow_mut().run_until_stalled();
        while cache().borrow().len() > 0 {
            // spawn after run
            while let Some(obj) = cache().borrow_mut().pop_front() {
                executor().borrow().spawner().spawn_local_obj(obj)?;
            }

            // run any new spawn task
            executor().borrow_mut().run_until_stalled();
        }
        epoller().borrow_mut().run()?;
    }
}
