use bson::DateTime;
use chrono::{Timelike, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use json_value_merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::arc2;
use crate::database::redis::UserSession;

/// Struct containing active players and spectators
pub struct Players {
    players: Arc<Mutex<HashSet<String>>>,
    spectators: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl Players {
    /// Adding one player
    pub fn add_player(&self, username: &str) -> usize {
        let mut players = self.players.lock().unwrap();
        players.insert(String::from(username));
        players.len()
    }

    /// Removing player
    pub fn remove_player(&mut self, username: &String) -> usize {
        let mut players = self.players.lock().unwrap();
        players.remove(username);
        players.len()
    }
}

impl Default for Players {
    fn default() -> Self {
        Self {
            players: arc2(HashSet::default()),
            spectators: arc2(HashMap::new()),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    username: String,
    time: String,
    msg: String,
}

impl ChatMsg {
    /// Creating new message
    fn new(username: String, time: String, msg: String) -> Self {
        Self {
            username,
            time,
            msg,
        }
    }

    /// Formats date in format HH:MM
    pub fn update(&mut self, user: &String) {
        let now = Utc::now().time();
        self.username = String::from(user);
        self.time = format!("{}:{}", now.hour(), now.minute());
    }

    /// Formats ChatMsg for json response.
    pub fn response(&mut self) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": "live_chat_message" });
        first.merge(second);
        first
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatRooms {
    messages: Arc<Mutex<HashMap<String, Vec<ChatMsg>>>>,
}

impl Default for ChatRooms {
    fn default() -> Self {
        let mut messages = HashMap::default();
        messages.insert(String::from("home"), vec![]);
        let messages = arc2(messages);
        Self { messages }
    }
}

impl ChatRooms {
    /// Can message be added to room.
    fn can_add(&self, chat: &Vec<ChatMsg>, player: &UserSession) -> bool {
        if !player.reg {
            return false;
        }
        let count = chat.iter().fold(0, |mut acc, x| {
            if &x.username == &player.username {
                acc += 1;
            }
            acc
        });
        if count < 5 {
            return true;
        }
        false
    }

    /// Check if message length less than 50 chars.
    fn message_length(&self, m: &ChatMsg) -> bool {
        if m.msg.len() > 0 && m.msg.len() < 50 {
            return true;
        }
        false
    }

    /// Format response for clients.
    fn response(&mut self) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": "live_chat_message" });
        first.merge(second);
        first
    }

    /// Add new message.
    pub fn add_msg(&self, id: &String, m: &mut ChatMsg, player: &UserSession) -> Option<Value> {
        if let Some(chat) = self.messages.lock().unwrap().get_mut(id) {
            if self.message_length(&m) {
                if self.can_add(&chat, player) {
                    m.update(&player.username);
                    let res = m.response();
                    chat.push(m.clone());
                    return Some(res);
                }
            }
        }
        None
    }
}
