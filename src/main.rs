use async_server::epoller::*;
use async_server::error::Result;
use async_server::runtime::RunTime;
use futures::StreamExt;
use std::cell::RefCell;
use std::rc::Rc;

async fn handle_conn(mut stream: AsyncTcpStream, epoller: Rc<RefCell<Epoller>>) {
    let mut buf: [u8; 1024] = [0; 1024];
    let shared_buf = Rc::new(RefCell::new(&mut buf[..]));
    stream
        .async_read(epoller.clone(), shared_buf)
        .for_each(|shared_buf| async move {
            println!("recv {}", String::from_utf8_lossy(&(*shared_buf).borrow()));
        })
        .await;
}

async fn accept_job(epoller: Rc<RefCell<Epoller>>) {
    let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();
    listener
        .async_accept(epoller.clone())
        .for_each_concurrent(None, |res| async {
            match res {
                Ok(conn) => handle_conn(conn, epoller.clone()).await,
                Err(err) => {
                    println!("err accept conn:{}", err);
                }
            }
        })
        .await;
}

fn main() -> Result<()> {
    let mut runtime = RunTime::new()?;
    runtime.spwan_job(accept_job)?;
    runtime.run()?;
    Ok(())
}
/*
fn main() -> Result<()> {
    let runtime = RunTime::new()?;
    let epoller = Rc::new(RefCell::new(Epoller::new().unwrap()));
    let mut executor = LocalPool::new();

    let copy = epoller.clone();
    executor
        .spawner()
        .spawn_local_obj(
            Box::new(async {
                let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();
                listener
                    .async_accept(copy)
                    .for_each_concurrent(None, |res| async {
                        match res {
                            Ok(stream) => println!("recv stream {:?}", stream),
                            Err(err) => println!("err occured:{}", err),
                        };
                        ready(()).await;
                    })
                    .await;
            })
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
*/

/*
async fn listen_task(
    epoller: Rc<RefCell<Epoller>>,
    handle_conn_callback: FnMut(Result<AsyncTcpStream>) -> Future<Output = ()>,
) -> Result<()> {
    let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();
    listener.async_accept(epoller).for_each_concurrent().await;
    Ok()
}
*/
