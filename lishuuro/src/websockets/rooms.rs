use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::arc2;
use crate::database::mongo::ShuuroGame;
use crate::database::redis::UserSession;

use super::server_messages::live_chat_message;

/// Struct containing active players and spectators
pub struct Players {
    online: Arc<Mutex<HashSet<String>>>,
    in_game: Arc<Mutex<HashSet<String>>>,
    spectators: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl Players {
    /// Adding one online player
    pub fn add_online_player(&self, username: &str) -> usize {
        let mut online = self.online.lock().unwrap();
        online.insert(String::from(username));
        online.len()
    }

    /// Adding players in game
    pub fn add_players(&self, players: &[String; 2]) -> usize {
        let mut in_game = self.in_game.lock().unwrap();
        in_game.insert(String::from(&players[0]));
        in_game.insert(String::from(&players[1]));
        in_game.len()
    }

    /// Removing players.
    pub fn remove_players(&self, players: &[String; 2]) -> usize {
        let mut in_game = self.in_game.lock().unwrap();
        in_game.remove(&players[0]);
        in_game.remove(&players[1]);
        in_game.len()
    }

    pub fn check_in_game(&self, username: &str) -> bool {
        let in_game = self.in_game.lock().unwrap();
        in_game.get(username).is_none()
    }

    /// Removing player
    pub fn remove_online_player(&self, username: &String) -> usize {
        let mut online = self.online.lock().unwrap();
        online.remove(username);
        online.len()
    }

    pub fn add_spectator(
        &self,
        id: &String,
        username: &String,
    ) -> Option<usize> {
        let mut spectators = self.spectators.lock().unwrap();
        if let Some(s) = spectators.get_mut(id) {
            s.insert(String::from(username));
            return Some(s.len());
        }
        None
    }

    pub fn remove_spectator(
        &self,
        id: &String,
        username: &String,
    ) -> Option<usize> {
        let mut spectators = self.spectators.lock().unwrap();
        if let Some(s) = spectators.get_mut(id) {
            s.remove(username);
            return Some(s.len());
        }
        None
    }

    /// Get spectators.
    pub fn get_spectators(&self, id: &str) -> Option<HashSet<String>> {
        if let Some(s) = self.spectators.lock().unwrap().get(&String::from(id))
        {
            return Some(s.clone());
        }
        None
    }

    pub fn get_online(&self) -> HashSet<String> {
        self.online.lock().unwrap().clone()
    }

    pub fn new_spectators(&self, id: &String) {
        self.spectators
            .lock()
            .unwrap()
            .insert(String::from(id), HashSet::new());
    }

    pub fn add_spectators(&self, games: &HashMap<String, ShuuroGame>) {
        for i in games.values() {
            self.new_spectators(&i._id);
        }
    }

    pub fn remove_spectators(&self, id: &String) {
        self.spectators.lock().unwrap().remove(id);
    }
}

impl Default for Players {
    fn default() -> Self {
        let mut spectators = HashMap::new();
        spectators.insert(String::from("home"), HashSet::new());
        spectators.insert(String::from("tv"), HashSet::new());
        Self {
            online: arc2(HashSet::default()),
            in_game: arc2(HashSet::default()),
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
    pub variant: String,
}

impl ChatMsg {
    /// Creating new message
    fn _new(user: String, message: String, id: String) -> Self {
        Self {
            user,
            time: bson::DateTime::now().to_string(),
            message,
            id,
            variant: String::from("shuuro"),
        }
    }

    /// Formats date in format HH:MM
    pub fn update(&mut self, user: &String) {
        self.user = String::from(user);
        self.time = chrono::offset::Local::now().to_rfc3339();
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
            if x.user == player.username {
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
        if !m.message.is_empty() && m.message.len() < 50 {
            return true;
        }
        false
    }

    /// Add new message.
    pub fn add_msg(
        &self,
        id: &String,
        mut m: ChatMsg,
        player: &UserSession,
    ) -> Option<Value> {
        if let Some(chat) = self.messages.lock().unwrap().get_mut(id) {
            if self.message_length(&m) && self.can_add(chat, player) {
                m.update(&player.username);
                let res = live_chat_message(&m);
                chat.push(m.clone());
                return Some(res);
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

    pub fn add_chat(&self, id: &String) {
        let mut chat = self.messages.lock().unwrap();
        chat.insert(String::from(id), vec![]);
        drop(chat);
    }

    pub fn remove_chat(&self, id: &String) {
        let mut chat = self.messages.lock().unwrap();
        chat.remove(&String::from(id));
        drop(chat);
    }
}
