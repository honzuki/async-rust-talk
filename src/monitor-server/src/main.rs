use rt_tokyo::{
    net::{TcpListener, TcpStream},
    Scheduler,
};

fn main() {
    let scheduler = Scheduler::default();
    scheduler.block_on(async_main());
}

async fn async_main() {
    let listener = TcpListener::bind("127.0.0.1:1663").unwrap();
    println!("server listening on: {}", listener.local_addr().unwrap());

    loop {
        let (conn, _) = listener.accept().await.unwrap();
        Scheduler::spawn(connection_handler(conn));
    }
}

async fn connection_handler(mut stream: TcpStream) {
    let mut buf = [0u8; 1024];
    while let Ok(rcount) = stream.read(&mut buf).await {
        if rcount == 0 {
            return;
        }

        println!("{}", String::from_utf8_lossy(&buf[..rcount]));
    }
}
