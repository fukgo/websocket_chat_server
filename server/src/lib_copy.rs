

use bcrypt::{hash, verify, DEFAULT_COST};
use futures::channel::mpsc::{self, UnboundedSender};
use futures::{future, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::hash::Hasher;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use WsMsg::*;
use rand;

pub async fn password_hash(password: &str) -> Result<String, bcrypt::BcryptError> {
    let hashed_password = hash(password, DEFAULT_COST)?;
    Ok(hashed_password)
}
use WsMsg::AuthMessage;
pub async fn auth_user(auth: &AuthMessage) -> Result<bool, bcrypt::BcryptError> {
    let mut user_hash: HashMap<String, String> = HashMap::new();
    user_hash.insert("admin".to_string(), password_hash("admin").await?);
    user_hash.insert("user".to_string(), password_hash("user").await?);
    if let Some(hashed_password) = user_hash.get(&auth.sender_username) {
        if verify(&auth.password, hashed_password)? {
            return Ok(true);
        } else {
            return Ok(false);
        }
    }
    Ok(false)
}

pub mod WsMsg{
    use bcrypt::{hash, verify, DEFAULT_COST};
    use futures::channel::mpsc::{self, UnboundedSender};
    use futures::{future, StreamExt, TryStreamExt};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::error::Error;
    use std::hash::Hasher;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::{Mutex, RwLock};
    use tokio_tungstenite::tungstenite::Message;
    use uuid::Uuid;
// WebSocket消息的不同类型
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    Auth,
    Enter,
    Leave,
    GroupChat,
    PrivateChat,
    PrivateInfo,
    GroupInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnterMessage {
    pub sender_username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeaveMessage {
    pub sender_username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupChatMessage {
    pub sender_username: String,
    pub content: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateChatMessage {
    pub target_username: String,
    pub sender_username: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthMessage {
    pub sender_username: String,
    pub password: String,
}
//info由服务器发出不需要客户端填写
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateInfoMessage {
    pub info: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupInfoMessage {
    pub info: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageBody {
    Auth(AuthMessage),
    Enter(EnterMessage),
    Leave(LeaveMessage),
    GroupChat(GroupChatMessage),
    PrivateChat(PrivateChatMessage),
    PrivateInfo(PrivateInfoMessage),
    GroupInfo(GroupInfoMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebSocketMessage {
    pub message_type: MessageType,
    pub message_body: MessageBody,
    pub token: Option<Uuid>,
}

impl WebSocketMessage {
    pub fn parse(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    //将WebSocketMessage转换为Message
    pub fn to_message(&self) -> Result<Message, serde_json::Error> {
        let json_str = self.to_json()?;
        Ok(Message::Text(json_str))
    }
}

}


async fn handle_connection(
    tcp_stream: tokio::net::TcpStream,
    peers: Arc<Mutex<HashMap<UserInfo, UnboundedSender<Message>>>>,
    peer_addr: SocketAddr,
    user_tokens: Arc<RwLock<HashMap<String, Uuid>>>,
) {
    // WebSocket 握手
    let ws_stream = tokio_tungstenite::accept_async(tcp_stream)
        .await
        .expect("握手时发生错误");
    println!("成功建立 WebSocket 连接: {}", peer_addr);

    let (msg_sender, msg_receiver) = mpsc::unbounded();
    let (write, read) = ws_stream.split();
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    let rng = rand::thread_rng();
    let random_string: String = rng
        .sample_iter(&Alphanumeric)
        .take(10)
        .take(10)
        .map(char::from)
        .collect();
    let random_username = format!("user-{}", random_string);

    // 将新的客户端加入到 peers 中
    {
        let mut peer_map = peers.lock().await;
        let user = UserInfo {
            source_addr: peer_addr,
            username: random_username,
            authenticated: false,
        };
        peer_map.insert(user, msg_sender);
    }
    println!("{} 连接成功", peer_addr);
    // 处理消息接收和广播
    let receive_and_broadcast_message = read.try_for_each(|message| {
        println!("接收到消息(Message)：{:?}", message);
        handle_message(message, peer_addr, peers.clone(), user_tokens.clone())
    });
    //将 unbounded 通道中的消息传递给 WebSocket 的写端 (write)
    let forward_messages = msg_receiver.map(Ok).forward(write);

    // 同时处理接收和发送任务，将两个异步任务并行运行。
    if let Err(e) = future::try_join(receive_and_broadcast_message, forward_messages).await {
        eprintln!("处理 WebSocket 消息时发生错误: {:?}", e);
    }
}

async fn send_broadcast_message(
    ws_msg: WebSocketMessage,
    sender: &mut UnboundedSender<Message>,
    target_vec: Vec<SocketAddr>,
) -> Result<(), Box<dyn Error>> {
    println!("vec:{:?}", target_vec);
    for target in target_vec {
        if sender.is_closed() {
            eprintln!("目标用户已经断开连接: {:?}", target);
            continue; // Skip to the next target
        }

        match sender.unbounded_send(ws_msg.clone().to_message().expect("转换消息时发生错误"))
        {
            Ok(_) => {}
            Err(err) => {
                eprintln!("发送消息失败: {:?}", err);
                continue; // Skip to the next target
            }
        }
    }
    Ok(())
}

pub async fn send_personal_message(
    ws_msg: WebSocketMessage,
    sender: &mut UnboundedSender<Message>,
    target: SocketAddr,
) {
    if !sender.is_closed() {
        match sender.unbounded_send(ws_msg.to_message().expect("转换消息时发生错误")) {
            Ok(_) => (),
            Err(err) => eprintln!("发送消息失败: {:?}", err),
        }
    } else {
        eprintln!("目标用户已经断开连接: {:?}", target);
    }
}

async fn handle_message(
    message: Message,
    peer_addr: SocketAddr,
    peers: Arc<Mutex<HashMap<UserInfo, UnboundedSender<Message>>>>,
    user_tokens: Arc<RwLock<HashMap<String, Uuid>>>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    match message {
        Message::Text(text_message) => {
            let ws_message = WebSocketMessage::parse(&text_message).expect("解析消息时发生错误");
            println!("解析消息: {:?}", ws_message);
            let mut addr_sender_vec: Vec<(UserInfo, UnboundedSender<Message>)>; // 提前声明

            // 锁住 peers 的同时，只获取相关数据
            {
                let peer_map = peers.lock().await;

                // 提取发送者和对应的地址到单独的 Vector 里，避免后面多次借用问题
                addr_sender_vec = peer_map
                    .iter()
                    .map(|(userinfo, sender)| (userinfo.clone(), sender.clone()))
                    .collect();
            } // 在这个作用域结束时，锁会自动释放

            // addr_sender_vec 已经在锁外，可以自由操作
            let mut addr_sender_vec_clone = addr_sender_vec.clone();

            // 查找对应的 peer_addr 的用户信息
            for (user, sender) in addr_sender_vec.iter_mut() {
                if peer_addr == user.source_addr {
                    match ws_message.message_type {
                        MessageType::Auth => {
                            if user.authenticated {
                                let auth_successed_msg = WebSocketMessage {
                                    message_type: MessageType::PrivateInfo,
                                    message_body: MessageBody::PrivateInfo(PrivateInfoMessage {
                                        info: "你已经登录认证成功，无需继续操作".to_string(),
                                    }),
                                    token: None,
                                };
                                send_personal_message(auth_successed_msg, sender, peer_addr).await;
                            } else {
                                if let MessageBody::Auth(ref auth) = ws_message.message_body {
                                    if auth_user(&auth).await.expect("认证时发生错误") {
                                        let mut user = user.clone();
                                        //修改peer全局的认证状态
                                        ///////!!!!!!!!!!!!!!!!!!!!!!!!
                                        let mut peer_map = peers.lock().await;
                                        let user_new = UserInfo {
                                            source_addr: user.source_addr.clone(),
                                            username: auth.sender_username.clone(),
                                            authenticated: true,
                                        };
                                        peer_map.remove(&user);
                                        peer_map.insert(user_new.clone(), sender.clone());

                                        let uuid: Uuid = Uuid::new_v4();
                                        let auth_success_msg = WebSocketMessage {
                                            message_type: MessageType::PrivateInfo,
                                            message_body: MessageBody::PrivateInfo(
                                                PrivateInfoMessage {
                                                    info: "auth-success".to_string(),
                                                },
                                            ),
                                            token: Some(uuid.clone()),
                                        };
                                        {
                                            user_tokens
                                                .write()
                                                .await
                                                .insert(user_new.username, uuid);
                                        }

                                        send_personal_message(auth_success_msg, sender, peer_addr)
                                            .await;
                                    } else {
                                        let auth_failed_msg = WebSocketMessage {
                                            message_type: MessageType::PrivateInfo,
                                            message_body: MessageBody::PrivateInfo(
                                                PrivateInfoMessage {
                                                    info: "登录认证失败,请检查账号或密码"
                                                        .to_string(),
                                                },
                                            ),
                                            token: None,
                                        };
                                        send_personal_message(auth_failed_msg, sender, peer_addr)
                                            .await;
                                    }
                                } else {
                                    eprintln!("收到无效的认证消息格式");
                                    let formated_error_msg = WebSocketMessage {
                                        message_type: MessageType::PrivateInfo,
                                        message_body: MessageBody::PrivateInfo(
                                            PrivateInfoMessage {
                                                info: "收到无效的认证消息格式".to_string(),
                                            },
                                        ),
                                        token: None,
                                    };
                                    send_personal_message(formated_error_msg, sender, peer_addr)
                                        .await;
                                }
                            }
                        }
                        _ => {
                            if user.authenticated {
                                {
                                    let user_right_token = user_tokens.read().await;

                                    println!("{:?}", user_right_token);
                                    if user_right_token.get(&user.username)
                                        != ws_message.token.as_ref()
                                    {
                                        let unauth_msg = WebSocketMessage {
                                            message_type: MessageType::PrivateInfo,
                                            message_body: MessageBody::PrivateInfo(
                                                PrivateInfoMessage {
                                                    info: "token错误,拒绝处理消息".to_string(),
                                                },
                                            ),
                                            token: None,
                                        };
                                        send_personal_message(unauth_msg, sender, peer_addr).await;
                                        return Ok(());
                                    }
                                    match ws_message.message_type {
                                        MessageType::Enter => {
                                            if let MessageBody::Enter(enter_message) =
                                                ws_message.message_body.clone()
                                            {
                                                let ws_msg = WebSocketMessage {
                                                    message_type: MessageType::GroupInfo,
                                                    message_body: MessageBody::GroupInfo(
                                                        GroupInfoMessage {
                                                            info: format!(
                                                                "{} 进入了聊天室",
                                                                enter_message.sender_username
                                                            ),
                                                        },
                                                    ),
                                                    token: None,
                                                };
                                                for (other_user, other_sender) in
                                                    addr_sender_vec_clone.iter_mut()
                                                {
                                                    send_personal_message(
                                                        ws_msg.clone(),
                                                        other_sender,
                                                        other_user.source_addr,
                                                    )
                                                    .await;
                                                }
                                            }
                                        }
                                        MessageType::Leave => {
                                            if let MessageBody::Leave(leave_message) =
                                                ws_message.message_body.clone()
                                            {
                                                let ws_msg = WebSocketMessage {
                                                    message_type: MessageType::GroupInfo,
                                                    message_body: MessageBody::GroupInfo(
                                                        GroupInfoMessage {
                                                            info: format!(
                                                                "{} 离开了聊天室",
                                                                leave_message.sender_username
                                                            ),
                                                        },
                                                    ),
                                                    token: None,
                                                };
                                                for (other_user, other_sender) in
                                                    addr_sender_vec_clone.iter_mut()
                                                {
                                                    send_personal_message(
                                                        ws_msg.clone(),
                                                        other_sender,
                                                        other_user.source_addr,
                                                    )
                                                    .await;
                                                }
                                            }
                                        }
                                        MessageType::GroupChat => {
                                            if let MessageBody::GroupChat(chat_message) =
                                                ws_message.message_body.clone()
                                            {
                                                let ws_msg = WebSocketMessage {
                                                    message_type: MessageType::GroupChat,
                                                    message_body: MessageBody::GroupChat(
                                                        GroupChatMessage {
                                                            sender_username: chat_message
                                                                .sender_username
                                                                .clone(),
                                                            content: format!(
                                                                "{}说：{}",
                                                                chat_message.sender_username,
                                                                chat_message.content
                                                            ),
                                                        },
                                                    ),
                                                    token: None,
                                                };
                                                for (other_user, other_sender) in
                                                    addr_sender_vec_clone.iter_mut()
                                                {
                                                    send_personal_message(
                                                        ws_msg.clone(),
                                                        other_sender,
                                                        other_user.source_addr,
                                                    )
                                                    .await;
                                                }
                                            }
                                        }
                                        MessageType::PrivateChat => {
                                            if let MessageBody::PrivateChat(chat_message) =
                                                ws_message.message_body.clone()
                                            {
                                                for (other_user, other_sender) in
                                                    addr_sender_vec_clone.iter_mut()
                                                {
                                                    if other_user.username
                                                        == chat_message.target_username
                                                    {
                                                        send_personal_message(
                                                            ws_message.clone(),
                                                            other_sender,
                                                            other_user.source_addr,
                                                        )
                                                        .await;
                                                    }
                                                }
                                            }
                                        }
                                        _ => eprintln!("不支持的数据类型"),
                                    }
                                }
                            } else {
                                let unauth_msg = WebSocketMessage {
                                    message_type: MessageType::PrivateInfo,
                                    message_body: MessageBody::PrivateInfo(PrivateInfoMessage {
                                        info: "未认证，拒绝处理消息".to_string(),
                                    }),
                                    token: None,
                                };
                                send_personal_message(unauth_msg, sender, peer_addr).await;
                            }
                        }
                    }
                }
            }
        }
        Message::Close(_) => {
            println!("{} 断开了连接", peer_addr);
        }
        _ => eprintln!("不支持的数据类型"),
    }

    Ok(())
}

#[derive(Eq, Clone, Debug)]
pub struct UserInfo {
    pub source_addr: SocketAddr,
    pub username: String,
    pub authenticated: bool,
}
impl PartialEq for UserInfo {
    fn eq(&self, other: &Self) -> bool {
        self.source_addr == other.source_addr
    }
}
use std::hash::Hash;
impl Hash for UserInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_addr.hash(state);
    }
}
use dashmap::DashMap;
pub struct ChatServer {
    // 将用户消息与其对应的 UnboundedSender 关联起来，用于给对应的用户发送消息
    peers: Arc<DashMap<UserInfo, UnboundedSender<Message>>>,
    // 将用户名与token关联起来，验证消息的合法性
    user_token: Arc<DashMap<String, Uuid>>,
}

impl ChatServer {
    pub fn new() -> Self {
        ChatServer {
            peers: Arc::new(Mutex::new(HashMap::new())),
            user_token: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&self, addr: SocketAddr){
        let tcp_listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        println!("服务器地址：{}", addr);

        while let Ok((stream, peer_addr)) = tcp_listener.accept().await {
            let peers = Arc::clone(&self.peers);
            let user_tokens = Arc::clone(&self.user_token);
            tokio::spawn(handle_connection(stream, peers, peer_addr, user_tokens));
        }
    }
}