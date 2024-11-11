use dashmap::DashMap;
use tokio_tungstenite::tungstenite::Message;
use crate::{message::*, user};
use futures::channel::mpsc::{UnboundedSender,UnboundedReceiver};
use tokio::sync::mpsc::{Sender,Receiver,self};
use uuid::Uuid;
use std::{any, net::SocketAddr, sync::Arc};
use crate::user::*;
use anyhow::{anyhow, bail, Context};
use tokio::net::TcpStream;
use log::{debug, error, log_enabled, info, Level};
use tokio::time::{timeout, Duration};
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::{StreamExt, SinkExt, future, TryFutureExt};
use futures_util::TryStreamExt;
use crate::handle_msg::*;


pub struct ChatServer {
    // 每个用户对应一个管道，用于接收消息
    peers: Arc<DashMap<User, Sender<Message>>>,
    // 每个认证用户携带一个token,<username:token>
    user_token: Arc<DashMap<String, Uuid>>,
    


}
impl ChatServer{
    pub fn new()->Arc<Self>{
        Arc::new(ChatServer{
            peers:Arc::new(DashMap::new()),
            user_token:Arc::new(DashMap::new()),
        })
    }
    pub async fn run(self:Arc<Self>, addr: SocketAddr) -> anyhow::Result<()> {
        let tcp_listener = tokio::net::TcpListener::bind(addr)
            .await
            .context(format!("Failed to bind to {}", addr))?;

        // 循环接收连接
        while let Ok((stream, addr)) = tcp_listener.accept().await {
            let this = Arc::clone(&self);
            // 在一个新的异步任务中处理连接
            tokio::spawn(async move {
                if let Err(e) = this.handle_connection(stream, addr).await {
                    eprintln!("Failed to handle connection: {:?}", e);
                }
            });
        }

        Ok(())
    }

    // 处理连接的函数，传递 TcpStream 和 peer_addr
    pub async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) ->anyhow::Result<()>{
        // 处理连接的具体逻辑
        println!("Handling connection from: {}", addr);
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        info!("WebSocket connection established: {}", addr);
        let (msg_sender, msg_receiver) = mpsc::channel(100);
        let (writer,reader) = ws_stream.split();
        // let ramdom_id = Uuid::new_v4();
        let user = User{
            // id:ramdom_id,
            source_addr:addr,
            username:generate_username().await,
            authenticated:false,
        };
        {
            self.peers.insert(user.clone(),msg_sender);
        }
        // 处理消息接收和广播
        let receive_and_broadcast = reader.try_for_each(|msg| {
            let peers = self.peers.clone();
            let user_token = self.user_token.clone();
            async move {
                if let Err(e) = handle_message(msg, addr, peers, user_token).await {
                    eprintln!("处理消息时发生错误: {:?}", e);
                    error!("处理消息时出错：{}", e);
                }
                Ok(())
            }
        });
        // 将通道中的消息传递给 WebSocket 的写端
        let msg_receiver_stream = ReceiverStream::new(msg_receiver);
        let send_to_ws = msg_receiver_stream.map(Ok).forward(writer).map_err(|e| {
            eprintln!("发送消息时发生错误: {:?}", e);
            error!("发送消息时出错：{}", e);
            e // 返回错误
        });
        // 同时处理接收和发送任务
        if let Err(e) = future::try_join(receive_and_broadcast, send_to_ws).await {
            eprintln!("处理 WebSocket 消息时发生错误: {:?}", e);
        }


        Ok(())
    }
    
    
}

pub async fn generate_username()->String{
    let rng = rand::thread_rng();
    let random_string: String = rng
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let random_username = format!("unauth_user-{}", random_string);
    random_username
}

pub async fn update_auth(peers: &Arc<DashMap<User, Sender<Message>>>, user: &User,username:&str) -> anyhow::Result<()> {
    /*
    将 User 的 authenticated 设置为 true
    */
    if let Some((old_user, sender)) = peers.remove(user) {
        let mut new_user = old_user;
        new_user.authenticated = true;
        new_user.username = username.to_string();
        peers.insert(new_user, sender); // 插入新的 User 键
        Ok(())
    } else {
        bail!("更新失败");
    }
}


pub async fn add_token(user_token: &Arc<DashMap<String, Uuid>>, id: Uuid, username: String) -> anyhow::Result<()> {
    user_token.insert(username, id);
    Ok(())
}

pub async fn remove_token(user_token: &Arc<DashMap<String, Uuid>>, username: &str) -> anyhow::Result<()> {
    user_token.remove(username);
    Ok(())
}
