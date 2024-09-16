
use bcrypt::{hash, verify, DEFAULT_COST};
use futures::channel::mpsc::{self, UnboundedSender};
use futures::{future, StreamExt, TryStreamExt};
use serde::de::value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::hash::Hasher;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use crate::message::*;
use crate::server::{add_token, update_auth};
use rand;
use dashmap::{DashMap, Entry};
use anyhow::Context;
use crate::user::*;
use tokio::sync::mpsc::Sender;
use log::{error,info,warn,debug};
use crate::user::*;
/*
    // 每个用户对应一个管道，用于接收消息
    peers: Arc<DashMap<User, Sender<Message>>>,
    // 每个认证用户携带一个token,<user_id:token>
    user_token: Arc<DashMap<Uuid, String>>,

*/
pub async fn handle_message(
    message: Message,
    peer_addr: SocketAddr,
    peers: Arc<DashMap<User, Sender<Message>>>,
    user_tokens: Arc<DashMap<String, Uuid>>,
) -> anyhow::Result<()> {
    match message {
        Message::Text(text_message) => {
            let ws_message = WebSocketMessage::parse(&text_message).with_context(|| {
                format!("解析 WebSocket 消息失败: {:?}", text_message)
            })?;
            // println!("解析消息: {:?}", ws_message);

            
            let mut addr_sender_vec: Vec<(User, Sender<Message>)> = peers
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect();
            debug!("处理来自 {} 的消息: {:?}", peer_addr, ws_message);
            debug!("当前连接的用户信息: {:?}", addr_sender_vec);

            // 查找对应的 peer_addr 的用户信息
            for (user, sender) in addr_sender_vec.iter_mut() {
                if peer_addr == user.source_addr {
                    debug!("找到匹配的用户: {:?}", user);
                    match ws_message.message_type {
                        MessageType::Auth => {
                            if user.authenticated {
                                info!("用户 {} 已经认证，无需重复认证", user.username);
                                let auth_successed_msg = WebSocketMessage {
                                    message_type: MessageType::PrivateInfo,
                                    message_body: MessageBody::PrivateInfo(PrivateInfoMessage {
                                        info: "你已经登录认证成功，无需继续操作".to_string(),
                                    }),
                                    token: None,
                                };
                                send_personal_message(auth_successed_msg, sender).await?;
                            } else {
                                if let MessageBody::Auth(ref auth) = ws_message.message_body {
                                    if auth_user(&auth).await.expect("认证时发生错误") {
                                        info!("用户 {} 认证成功", user.username);
                                        update_auth(&peers, &user,&auth.sender_username).await?;
                                        let uuid = Uuid::new_v4();
                                        let _ = add_token(&user_tokens, uuid, user.username.clone()).await;

                                        let auth_success_msg = WebSocketMessage {
                                            message_type: MessageType::PrivateInfo,
                                            message_body: MessageBody::PrivateInfo(
                                                PrivateInfoMessage {
                                                    info: "auth-success".to_string(),
                                                },
                                            ),
                                            token: Some(uuid.clone()),
                                        };

                                        send_personal_message(auth_success_msg, sender).await?;
                                    } else {
                                        error!("用户 {} 认证失败", user.username);
                                        let auth_failed_msg = WebSocketMessage {
                                            message_type: MessageType::PrivateInfo,
                                            message_body: MessageBody::PrivateInfo(
                                                PrivateInfoMessage {
                                                    info: "登录认证失败,请检查账号或密码".to_string(),
                                                },
                                            ),
                                            token: None,
                                        };
                                        send_personal_message(auth_failed_msg, sender).await?;
                                    }
                                } else {
                                    error!("收到无效的认证消息格式: {:?}", ws_message.message_body);
                                    let formated_error_msg = WebSocketMessage {
                                        message_type: MessageType::PrivateInfo,
                                        message_body: MessageBody::PrivateInfo(
                                            PrivateInfoMessage {
                                                info: "收到无效的认证消息格式".to_string(),
                                            },
                                        ),
                                        token: None,
                                    };
                                    send_personal_message(formated_error_msg, sender).await?;
                                }
                            }
                        }
                        _ => {
                            if user.authenticated {
                                if let Some(ref entry) = user_tokens.get(&user.username) {
                                    if entry.value() != &ws_message.token.unwrap() {
                                        error!("用户 {} 的 token 不匹配", user.username);
                                        let unauth_msg = WebSocketMessage {
                                            message_type: MessageType::PrivateInfo,
                                            message_body: MessageBody::PrivateInfo(
                                                PrivateInfoMessage {
                                                    info: "token错误,拒绝处理消息".to_string(),
                                                },
                                            ),
                                            token: None,
                                        };
                                        send_personal_message(unauth_msg, sender).await?;
                                        return Ok(());
                                    }
                                }

                                match ws_message.message_type {
                                    MessageType::Enter => {
                                        if let MessageBody::Enter(enter_message) = ws_message.message_body.clone() {
                                            let ws_msg = WebSocketMessage {
                                                message_type: MessageType::GroupInfo,
                                                message_body: MessageBody::GroupInfo(GroupInfoMessage {
                                                    info: format!("{} 进入了聊天室", enter_message.sender_username),
                                                }),
                                                token: None,
                                            };
                                            debug!("用户 {} 进入聊天室", enter_message.sender_username);
                                            send_group_message(ws_msg, peers.clone()).await?;
                                        }
                                    }
                                    MessageType::Leave => {
                                        if let MessageBody::Leave(leave_message) = ws_message.message_body.clone() {
                                            let ws_msg = WebSocketMessage {
                                                message_type: MessageType::GroupInfo,
                                                message_body: MessageBody::GroupInfo(GroupInfoMessage {
                                                    info: format!("{} 离开了聊天室", leave_message.sender_username),
                                                }),
                                                token: None,
                                            };
                                            debug!("用户 {} 离开聊天室", leave_message.sender_username);
                                            send_group_message(ws_msg, peers.clone()).await?;
                                        }
                                    }
                                    MessageType::GroupChat => {
                                        if let MessageBody::GroupChat(chat_message) = ws_message.message_body.clone() {
                                            let ws_msg = WebSocketMessage {
                                                message_type: MessageType::GroupChat,
                                                message_body: MessageBody::GroupChat(GroupChatMessage {
                                                    sender_username: chat_message.sender_username.clone(),
                                                    content: format!(
                                                        "{}说：{}",
                                                        chat_message.sender_username,
                                                        chat_message.content
                                                    ),
                                                }),
                                                token: None,
                                            };
                                            debug!("用户 {} 发送群聊消息", chat_message.sender_username);
                                            send_group_message(ws_msg, peers.clone()).await?;
                                        }
                                    }
                                    MessageType::PrivateChat => {
                                        if let MessageBody::PrivateChat(chat_message) = ws_message.message_body.clone() {
                                            let ws_msg = WebSocketMessage {
                                                message_type: MessageType::PrivateChat,
                                                message_body: MessageBody::PrivateChat(PrivateChatMessage {
                                                    target_username: chat_message.target_username.clone(),
                                                    sender_username: chat_message.sender_username.clone(),
                                                    content: format!(
                                                        "{}对你说：{}",
                                                        chat_message.sender_username,
                                                        chat_message.content
                                                    ),
                                                }),
                                                token: None,
                                            };
                                            debug!("用户 {} 向 {} 发送私聊消息", chat_message.sender_username, chat_message.target_username);
                                            send_private_message(ws_msg, peers.clone(), chat_message.target_username.clone()).await?;
                                        }
                                    }
                                    _ => {
                                        error!("不支持的数据类型: {:?}", ws_message.message_type);
                                    }
                                }
                            } else {
                                error!("用户 {} 未认证，拒绝处理消息", user.username);
                                let unauth_msg = WebSocketMessage {
                                    message_type: MessageType::PrivateInfo,
                                    message_body: MessageBody::PrivateInfo(PrivateInfoMessage {
                                        info: "未认证，拒绝处理消息".to_string(),
                                    }),
                                    token: None,
                                };
                                send_personal_message(unauth_msg, sender).await?;
                            }
                        }
                    }
                }
            }
        }
        Message::Close(_) => {
            println!("{} 断开了连接", peer_addr);
        }
        _ => {
            error!("不支持的数据类型: {:?}", message);
        }
    }

    Ok(())
}



pub async fn send_private_message(
    ws_msg: WebSocketMessage,
    peers: Arc<DashMap<User, Sender<Message>>>,
    target_username: String,
) -> anyhow::Result<()> {
    // 遍历 peers
    for entry in peers.iter() {
        let user = entry.key();
        let sender = entry.value();
        // 查找目标用户
        if user.username == target_username {
            // 检查是否连接还没有关闭
            if !sender.is_closed() {
                // 尝试发送消息
                sender.send(ws_msg.to_message()?).await?;
            } else {
                error!("发送消息失败，发送者已关闭");
                return Err(anyhow::anyhow!("发送消息失败，发送者已关闭"));
            }
        }
    }

    Ok(())
}



pub async fn send_personal_message(
    ws_msg: WebSocketMessage,
    sender:&mut Sender<Message>,
    // target:SocketAddr,
)->anyhow::Result<()>{
    if !sender.is_closed(){
        sender.send(ws_msg.to_message()?).await?;
        Ok(())
    }else {
        error!("发送消息失败，发送者已关闭");
        Err(anyhow::anyhow!("发送消息失败，发送者已关闭"))
    }
}
pub async fn send_group_message(
    ws_msg: WebSocketMessage,
    peers: Arc<DashMap<User, Sender<Message>>>,
)->anyhow::Result<()>{
    for map in peers.iter_mut() {
        let sender = map.value();
        if !sender.is_closed(){
            sender.send(ws_msg.to_message()?).await?;
        }else {
            error!("发送消息失败，发送者已关闭");
            return Err(anyhow::anyhow!("发送消息失败，发送者已关闭"));
        }
    }
    Ok(())
}