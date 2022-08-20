use serde_json::Value;

use crate::database::redis::UserSession;

#[derive(Clone)]
pub struct ClientMessage {
    pub reg: bool,
    pub username: String,
    pub msg: Value,
}

impl ClientMessage {
    pub fn new(session: &UserSession, msg: Value) -> Self {
        Self {
            reg: session.reg,
            username: String::from(&session.username),
            msg,
        }
    }
}

pub enum ChannelMessage {
    Json(ClientMessage),
    NotJson(String),
}

impl ChannelMessage {
    pub fn json(c: ClientMessage) -> Self {
        Self::Json(c)
    }

    pub fn not_json(s: &str) -> Self {
        Self::NotJson(String::from(s))
    }
}
