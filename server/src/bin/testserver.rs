use log::warn;
use server::server::ChatServer;

#[tokio::main]
async fn main() {
    env_logger::init();
    let addr = "0.0.0.0:8080".parse().unwrap();
    let server = ChatServer::new();
    if let Err(e) = server.run(addr).await{
        warn!("发送错误:{}",e);
        eprint!("{}",e)
    };
}