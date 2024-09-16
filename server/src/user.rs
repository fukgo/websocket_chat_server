use std::net::SocketAddr;
use std::hash::Hash;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use crate::message::*;
use std::collections::HashMap;
use bcrypt::verify;
use bcrypt::hash;
use bcrypt::DEFAULT_COST;
#[derive(Eq, Clone, Debug,Hash)]
pub struct User {
    // pub id:Uuid,
    pub source_addr: SocketAddr,
    pub username: String,
    pub authenticated: bool,
}
// 把User的id当作主键
impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.source_addr == other.source_addr
    }

}

pub struct Token{
    pub value:String,
    pub expires_at: u64, // Unix 时间戳

}
fn generate_token(hour:u64) -> Token {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let expires_at = now + hour*3600; // Token n 小时后过期

    Token {
        value: Uuid::new_v4().to_string(),
        expires_at,
    }
}
// 验证Token是否有效
fn validate_token(token: &Token) -> bool {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    if now < token.expires_at {
        println!("Token is valid");
        true
    } else {
        println!("Token has expired");
        false
    }
}
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
pub async fn password_hash(password: &str) -> Result<String, bcrypt::BcryptError> {
    let hashed_password = hash(password, DEFAULT_COST)?;
    Ok(hashed_password)
}