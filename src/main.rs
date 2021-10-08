use async_server::epoller::*;
use async_server::error::Result;
use async_server::runtime;
//use futures::StreamExt;
//use std::cell::RefCell;
//use std::rc::Rc;

async fn echo_task() {
    let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();
    loop {
        let mut stream = listener.async_accept().await.unwrap();
        println!("new conn:{:?}", stream);
        runtime::spawn_task(async move {
            let mut buf: [u8; 1024] = [0; 1024];
            let mut cnt = stream.async_read(&mut buf[..]).await.unwrap();
            while cnt != 0 {
                println!("recv: {}", String::from_utf8_lossy(&buf[0..cnt]));
                cnt = stream.async_read(&mut buf[..]).await.unwrap();
            }
            //stream.async_write(&buf[0..cnt]).await.unwrap();
        })
        .unwrap();
    }
}

fn main() -> Result<()> {
    runtime::spawn_task(echo_task())?;
    runtime::run()?;
    Ok(())
}
