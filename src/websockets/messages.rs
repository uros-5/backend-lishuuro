use std::sync::Arc;

use serde_json::Value;
use tokio::sync::broadcast::Sender;

use crate::database::redis::UserSession;

use super::WsState;

#[derive(Clone)]
pub struct ClientMessage {
    pub reg: bool,
    pub username: String,
    pub msg: Value,
    pub to: SendTo,
}

impl ClientMessage {
    pub fn new(session: &UserSession, msg: Value, to: SendTo) -> Self {
        Self {
            reg: session.reg,
            username: String::from(&session.username),
            msg,
            to,
        }
    }
}

pub type NewMessage = (
    &'static Arc<WsState>,
    &'static UserSession,
    &'static Sender<ClientMessage>,
);

#[derive(Clone)]
pub enum SendTo {
    Me,
    All,
    Spectators(Vec<String>),
    Players([String; 2]),
    SpectatorsAndPlayers((Vec<String>, [String; 2])),
}

pub fn onconnect(ws: &Arc<WsState>, user: &UserSession, tx: &Sender<ClientMessage>, con: bool) {
    let count: usize = {
        if con {
            ws.players.add_player(&user.username)
        } else {
            ws.players.remove_player(&user.username)
        }
    };
    let value = serde_json::json!({ "t": "active_players_count", "cnt": count });
    let cm = ClientMessage::new(user, value, SendTo::All);
    tx.send(cm);

    if con {
        let chat = ws.chat.get_chat(&String::from("home"));
        let value = serde_json::json!({ "t": "live_chat_full", "id": "home", "lines": chat});
        let cm = ClientMessage::new(user, value, SendTo::Me);
        tx.send(cm);
    }
}
