use server::ChatServer;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:8080".parse().unwrap();
    let server = ChatServer::new();
    server.run(addr).await;
}