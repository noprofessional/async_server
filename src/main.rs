use async_server::epoller::*;
use async_server::error::Result;
use async_server::runtime;
//use futures::StreamExt;
//use std::cell::RefCell;
//use std::rc::Rc;

// provide service on fixed port
async fn echo_service() {
    let listener = AsyncTcpListener::bind("0.0.0.0:23333").unwrap();
    loop {
        let stream = listener.async_accept().await.unwrap();
        println!("new conn:{:?}", stream);

        // create new task for every connection
        runtime::spawn_task(echo_task(stream)).unwrap();
    }
}

async fn echo_task(mut stream: AsyncTcpStream) {
    let mut buf: [u8; 1024] = [0; 1024];
    loop {
        let cnt = stream.async_read(&mut buf[..]).await.unwrap();
        if cnt != 0 {
            println!("recv: {}", String::from_utf8_lossy(&buf[0..cnt]));
        } else {
            // connection breaks
            println!("closed:{:?}", stream);
            break;
        }
    }
}

fn main() -> Result<()> {
    runtime::spawn_task(echo_service())?;
    runtime::run()?;
    Ok(())
}
