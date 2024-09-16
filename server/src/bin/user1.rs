use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Write;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;
pub enum WsError {
    ParseError(serde_json::Error),
    AuthError(String),
}

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

pub struct UserStatus {
    pub username: Option<String>,
    pub is_auth: bool,
    pub in_room: bool,
}

impl UserStatus {
    pub fn display(&self) {
        println!("用户名: {:?}", self.username);
        println!("登录状态: {:?}", self.is_auth);
        println!("进入聊天室: {:?}", self.in_room);
    }
}

async fn send_msg(sender: &mut UnboundedSender<Message>, ws_msg: WebSocketMessage) {
    let msg = ws_msg.to_message().unwrap();
    sender.send(msg).await.unwrap();
}

async fn handle_receive(
    mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<(), Box<dyn Error>> {
    while let Some(msg) = reader.next().await {
        let ws_msg = WebSocketMessage::parse(msg.unwrap().to_text().unwrap()).expect("parse error");
        println!("接受ws_msg: {:?}", ws_msg);
        match ws_msg.message_type {
            MessageType::Enter => {
                let enter_msg = ws_msg.message_body;
                match enter_msg {
                    MessageBody::Enter(enter_msg) => {
                        println!("{} 进入了聊天室", enter_msg.sender_username);
                    }
                    _ => {
                        println!("e")
                    }
                }
            }
            MessageType::Leave => {
                let leave_msg = ws_msg.message_body;
                match leave_msg {
                    MessageBody::Leave(leave_msg) => {
                        println!("{} 离开了聊天室", leave_msg.sender_username);
                    }
                    _ => {
                        println!("e")
                    }
                }
            }
            MessageType::GroupChat => {
                let group_chat_msg = ws_msg.message_body;
                match group_chat_msg {
                    MessageBody::GroupChat(group_chat_msg) => {
                        println!(
                            "(群聊){}: {}",
                            group_chat_msg.sender_username, group_chat_msg.content
                        );
                    }
                    _ => {
                        println!("e")
                    }
                }
            }
            MessageType::PrivateChat => {
                let private_chat_msg = ws_msg.message_body;
                match private_chat_msg {
                    MessageBody::PrivateChat(private_chat_msg) => {
                        println!(
                            "(私聊){} to 'me': {}",
                            private_chat_msg.sender_username, private_chat_msg.content
                        );
                    }
                    _ => {
                        println!("e")
                    }
                }
            }
            MessageType::PrivateInfo => {
                let private_info_msg = ws_msg.message_body;
                match private_info_msg {
                    MessageBody::PrivateInfo(private_info) => {
                        println!("(通知): {}", private_info.info);
                    }
                    _ => {
                        println!("e")
                    }
                }
            }
            MessageType::GroupInfo => {
                let group_info_msg = ws_msg.message_body;
                match group_info_msg {
                    MessageBody::GroupInfo(group_info) => {
                        println!("(通知): {}", group_info.info);
                    }
                    _ => {
                        println!("e")
                    }
                }
            }

            _ => {
                println!("e")
            }
        }
    }
    Ok(())
}

use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;

async fn await_token(
    reader: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<Option<Uuid>, Box<dyn Error>> {
    if let Some(msg) = reader.next().await {
        let msg = msg?;
        println!("第一次接受msg:{}", msg);
        let ws_msg = WebSocketMessage::parse(&msg.to_text()?)?;
        match ws_msg.message_body {
            MessageBody::PrivateInfo(auth_msg) => {
                let token = ws_msg.token;
                return Ok(token);
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}
use futures::stream::SplitSink;

async fn input_user() -> (WebSocketMessage, String) {
    let mut username = String::new();
    print!("请输入用户名: ");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut username).unwrap();

    let mut password = String::new();
    print!("请输入密码: ");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut password).unwrap();

    let username = username.trim().to_string();
    let password = password.trim().to_string();
    println!("username: {}", username);
    println!("password: {}", password);
    let auth_msg = WebSocketMessage {
        message_type: MessageType::Auth,
        message_body: MessageBody::Auth(AuthMessage {
            sender_username: username.clone(),
            password: password.to_string(),
        }),
        token: None,
    };
    (auth_msg, username)
}
async fn handle_input(
    mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut user_status: UserStatus,
    token_uuid: Option<Uuid>,
) -> Result<(), Box<dyn Error>> {
    loop {
        print!("{:?}", token_uuid);
        let mut input_type = String::new();
        println!("Please input your msg_type: \n in:进入聊天室 \n out:退出聊天室 \n group:群聊 \n private:私聊 \n 输入你的选择：");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input_type).unwrap();

        match input_type.trim().to_lowercase().as_str() {
            "in" => {
                if user_status.in_room {
                    println!("您已经在聊天室中，请勿重复进入");
                    continue;
                }
                let enter_msg = WebSocketMessage {
                    message_type: MessageType::Enter,
                    message_body: MessageBody::Enter(EnterMessage {
                        sender_username: user_status.username.clone().unwrap(),
                    }),
                    token: token_uuid,
                };
                if let Err(e) = writer.send(enter_msg.to_message()?).await {
                    eprintln!("发送进入聊天室消息时发生错误: {:?}", e);
                }
                user_status.in_room = true;
                println!("进入聊天室成功");
            }
            "out" => {
                if !user_status.in_room {
                    println!("您已经退出聊天室，请勿重复退出");
                    continue;
                }
                let leave_msg = WebSocketMessage {
                    message_type: MessageType::Leave,
                    message_body: MessageBody::Leave(LeaveMessage {
                        sender_username: user_status.username.clone().unwrap(),
                    }),
                    token: token_uuid,
                };
                if let Err(e) = writer.send(leave_msg.to_message()?).await {
                    eprintln!("发送退出聊天室消息时发生错误: {:?}", e);
                }
                user_status.in_room = false;
                println!("退出聊天室成功");
            }
            "group" => {
                if !user_status.in_room {
                    println!("您还未进入聊天室，请先进入聊天室");
                    continue;
                }
                let mut content = String::new();
                print!("Please input your content: ");
                std::io::stdout().flush().unwrap();
                std::io::stdin().read_line(&mut content).unwrap();
                let group_msg = WebSocketMessage {
                    message_type: MessageType::GroupChat,
                    message_body: MessageBody::GroupChat(GroupChatMessage {
                        sender_username: user_status.username.clone().unwrap(),
                        content: content.trim().to_string(),
                    }),
                    token: token_uuid,
                };
                if let Err(e) = writer.send(group_msg.to_message()?).await {
                    eprintln!("发送群聊消息时发生错误: {:?}", e);
                }
                println!("发送群聊消息成功");
            }
            "private" => {
                if user_status.in_room {
                    println!("您已经在聊天室中，无法进行私聊");
                    continue;
                }
                let mut target_username = String::new();
                print!("Please input your target_username: ");
                std::io::stdout().flush().unwrap();
                std::io::stdin().read_line(&mut target_username).unwrap();

                let mut content = String::new();
                print!("Please input your content: ");
                std::io::stdout().flush().unwrap();
                std::io::stdin().read_line(&mut content).unwrap();
                let private_msg = WebSocketMessage {
                    message_type: MessageType::PrivateChat,
                    message_body: MessageBody::PrivateChat(PrivateChatMessage {
                        target_username: target_username.trim().to_string(),
                        sender_username: user_status.username.clone().unwrap(),
                        content: content.trim().to_string(),
                    }),
                    token: token_uuid,
                };
                if let Err(e) = writer.send(private_msg.to_message()?).await {
                    eprintln!("发送私聊消息时发生错误: {:?}", e);
                }
                println!("发送私聊消息成功");
            }
            _ => {
                println!("输入错误，请重新输入");
            }
        }
    }
}

use tokio::task;

use futures_util::stream::SplitStream;
use futures_util::future;

async fn run(url: &str) {
    let (ws_stream, _response) = connect_async(url).await.expect("连接失败");
    let (mut writer, mut reader) = ws_stream.split();
    let mut user_status = UserStatus {
        username: None,
        is_auth: false,
        in_room: false,
    };

    // 用户输入用户名和密码，获取认证消息
    let (auth_msg, username) = input_user().await;
    
    // 发送认证消息
    writer
        .send(auth_msg.to_message().expect("消息序列化失败"))
        .await
        .expect("发送失败");

    // 等待服务器返回token
    let token = await_token(&mut reader).await.expect("获取token失败");
    println!("token: {:?}", token);
    
    if let Some(token) = token {
        user_status.username = Some(username);
        user_status.is_auth = true;
        println!("欢迎，认证成功");
    } else {
        eprintln!("认证失败");
        return;
    }

    // 创建任务来处理接收和发送消息
    let receive_handle = task::spawn(async move {
        if let Err(e) = handle_receive(reader).await {
            eprintln!("处理接收消息时发生错误: {:?}", e);
        }
    });

    let input_handle = task::spawn(async move {
        if let Err(e) = handle_input(writer, user_status, token).await {
            eprintln!("处理发送消息时发生错误: {:?}", e);
        }
    });

    // 并行运行接收和输入任务
    if let Err(e) = future::try_join(receive_handle, input_handle).await {
        eprintln!("WebSocket任务失败: {:?}", e);
    }
}


#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:8080";
    run(url).await;
}
