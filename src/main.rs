use async_server::epoller::*;
use futures::StreamExt;
//use futures::executor::block_on_stream;
use futures::executor::LocalPool;
use futures::task::LocalSpawn;
use std::cell::RefCell;
use std::future::ready;
use std::io::Result;
use std::rc::Rc;

fn main() -> Result<()> {
    let epoller = Rc::new(RefCell::new(Epoller::new().unwrap()));
    let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();

    /*
    // this only create a new rc points to same refcell
    // refcell is not cloned
    let copy = epoller.clone();

    epoller
        .borrow()
        .spwan(async move {
            let stream = listener.async_accept(copy).await.unwrap();
            println!("recv stream {:?}", stream);
        })
        .unwrap();
    */

    let copy = epoller.clone();
    let mut executor = LocalPool::new();
    executor
        .spawner()
        .spawn_local_obj(
            Box::new(
                listener
                    .async_accept(copy)
                    .for_each_concurrent(None, |res| async {
                        match res {
                            Ok(stream) => println!("recv stream {:?}", stream),
                            Err(err) => println!("err occured:{}", err),
                        };
                        ready(()).await;
                    }),
            )
            .into(),
        )
        .unwrap();

    let stop = false;
    while !stop {
        executor.run_until_stalled();
        epoller.borrow_mut().run()?;
    }
    Ok(())

    /*
    while let Some(result) = block_on_stream(listener.async_accept()).next() {
        match result {
            Ok(tcpstream) => {
                println!("new stream:{:?}", tcpstream);
                break;
            }
            Err(err) => {
                println!("err:{}", err);
                break;
            }
        }
    }
    */
}

/*
async fn handle_connection(stream: AsyncTcpStream) -> Result<()> {
    let data = stream.async_read(epoller).await?;
    stream.async_write(epoller).await?;
}
*/

async fn listen_task(
    epoller: Rc<RefCell<Epoller>>,
    handle_conn_callback: FnMut(Result<AsyncTcpStream>) -> Future<Output = ()>,
) -> Result<()> {
    let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();
    listener.async_accept(epoller).for_each_concurrent().await;
    Ok()
}
