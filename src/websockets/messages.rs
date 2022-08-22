use std::{sync::Arc, collections::HashSet};

use async_session::blake3::Hash;
use serde_json::Value;
use tokio::sync::broadcast::Sender;

use crate::database::redis::UserSession;

use super::{rooms::ChatMsg, WsState};

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
    Spectators(HashSet<String>),
    Players([String; 2]),
    SpectatorsAndPlayers((HashSet<String>, [String; 2])),
}

pub fn connecting(ws: &Arc<WsState>, user: &UserSession, tx: &Sender<ClientMessage>, con: bool) {
    let count: usize = {
        if con {
            ws.players.add_spectator(&String::from("home"), &user.username);
            ws.players.add_player(&user.username)
        } else {
            ws.players.remove_spectator(&String::from("home"), &user.username);
            ws.players.remove_spectator(&user.watches, &user.username);
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

pub fn new_chat_msg(
    ws: &Arc<WsState>,
    user: &UserSession,
    tx: &Sender<ClientMessage>,
    msg: &mut ChatMsg,
) {
    let id = String::from(&msg.id);
    if let Some(v) = ws.chat.add_msg(&id, msg, &user) {
        if let Some(s) = ws.players.get_spectators(&msg.id) {
            let to: SendTo;
            if &msg.id == "home" {
                to = SendTo::Spectators(s);
            } else {
                to = SendTo::SpectatorsAndPlayers((s, [String::from(""), String::from("")]));
            }
            let cm = ClientMessage::new(user, v, to);
            tx.send(cm);
        }
    }
}
