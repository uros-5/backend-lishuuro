use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use json_value_merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::arc2;

pub const VARIANTS: [&str; 2] = ["shuuro12", "shuuro12fairy"];
pub const DURATION_RANGE: [i64; 28] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30, 35, 40, 45, 60,
    75, 90,
];

#[derive(Clone, Serialize, Deserialize)]
pub struct GameRequest {
    pub username: String,
    pub variant: String,
    pub time: i64,
    pub incr: i64,
    color: String,
}

impl GameRequest {
    /// Return true if game has valid time.
    pub fn is_valid(&self) -> bool {
        if VARIANTS.contains(&self.variant.as_str()) {
            if DURATION_RANGE.contains(&self.time) {
                if DURATION_RANGE.contains(&self.incr) {
                    return true;
                } else if &self.incr == &0 {
                    return true;
                }
            }
        }
        false
    }

    /// Formats game for json response.
    pub fn response(&mut self, t: &String) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": t });
        first.merge(second);

        first
    }

    /// Return id for game
    pub fn username(&self) -> String {
        String::from(&self.username)
    }

    /// Returns player colors
    pub fn colors(&self, other: &String) -> [String; 2] {
        let c_s: [String; 2];
        let mut color = String::from("");
        if &self.color == "random" {
            color = self.random_color();
        }
        if color == "white" {
            c_s = [String::from(&self.username), String::from(other)];
        } else {
            c_s = [String::from(other), String::from(&self.username)];
        }
        c_s
    }

    /// Generate random color.
    fn random_color(&self) -> String {
        if rand::random() {
            String::from("white")
        } else {
            String::from("black")
        }
    }
}

pub struct GameReqs {
    all: Arc<Mutex<HashMap<String, GameRequest>>>,
}

impl Default for GameReqs {
    fn default() -> Self {
        Self {
            all: arc2(HashMap::new()),
        }
    }
}

impl GameReqs {
    /// Add GameRequest to struct.
    pub fn add(&self, mut game: GameRequest) -> Option<Value> {
        let mut all = self.all.lock().unwrap();
        if !all.contains_key(&game.username) {
            if game.is_valid() {
                let res = game.response(&String::from("home_lobby_add"));
                all.insert(String::from(&game.username), game.clone());
                return Some(res);
            }
        }
        None
    }

    /// Remove game from struct.
    pub fn remove(&self, t: &str, username: &String) -> Option<Value> {
        let mut all = self.all.lock().unwrap();
        if let Some(mut game) = all.remove(username) {
            let res = game.response(&String::from(t));
            return Some(res);
        }
        None
    }

    /// Get all game requests.
    pub fn get_all(&self) -> Vec<GameRequest> {
        let all = self.all.lock().unwrap();
        let mut g = vec![];
        for i in all.values() {
            g.push(i.clone());
        }
        g
    }

    /// Generate response for one GameRequests
    pub fn response(&self, all: Vec<GameRequest>) -> Value {
        json!({ "t": "home_lobby_full", "lobbyGames": all})
    }
}
