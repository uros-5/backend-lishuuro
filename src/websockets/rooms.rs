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
    spectators: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl Players {
    /// Adding one player
    pub fn add_player(&self, username: &str) -> usize {
        let mut players = self.players.lock().unwrap();
        players.insert(String::from(username));
        players.len()
    }

    /// Removing player
    pub fn remove_player(&self, username: &String) -> usize {
        let mut players = self.players.lock().unwrap();
        players.remove(username);
        players.len()
    }

    pub fn add_spectator(&self, id: &String, username: &String) -> Option<usize> {
        let mut spectators = self.spectators.lock().unwrap();
        if let Some(s) = spectators.get_mut(id) {
            s.insert(String::from(username));
            return Some(s.len());
        }
        None
    }

    pub fn remove_spectator(&self, id: &String, username: &String) -> Option<usize> {
        let mut spectators = self.spectators.lock().unwrap();
        if let Some(s) = spectators.get_mut(id) {
            s.remove(username);
            return Some(s.len());
        }
        None
    }

    /// Get spectators.
    pub fn get_spectators(&self, id: &str) -> Option<HashSet<String>> {
        if let Some(s) = self.spectators.lock().unwrap().get(&String::from(id)) {
            return Some(s.clone());
        }
        None
    }

    pub fn get_players(&self) -> HashSet<String> {
        self.players.lock().unwrap().clone()
    }
}

impl Default for Players {
    fn default() -> Self {
        let mut spectators = HashMap::new();
        spectators.insert(String::from("home"), HashSet::new());
        Self {
            players: arc2(HashSet::default()),
            spectators: arc2(spectators),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    pub id: String,
    pub user: String,
    pub time: String,
    pub message: String,
}

impl ChatMsg {
    /// Creating new message
    fn _new(user: String, time: String, message: String, id: String) -> Self {
        Self {
            user,
            time,
            message,
            id,
        }
    }

    /// Formats date in format HH:MM
    pub fn update(&mut self, user: &String) {
        let now = Utc::now().time();
        self.user = String::from(user);
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
            if &x.user == &player.username {
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
        if m.message.len() > 0 && m.message.len() < 50 {
            return true;
        }
        false
    }

    /// Format response for clients.
    fn _response(&mut self) -> Value {
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

    pub fn get_chat(&self, id: &String) -> Option<Vec<ChatMsg>> {
        let chat = self.messages.lock().unwrap();
        if let Some(chat) = chat.get(id) {
            return Some(chat.clone());
        }
        None
    }
}
