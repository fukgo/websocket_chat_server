use serde::{Deserialize, Serialize};
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
