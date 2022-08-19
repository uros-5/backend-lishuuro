use json_value_merge::Merge;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use crate::arc2;

use super::rooms::{ChatRooms, Players};
use tokio::sync::broadcast;

pub const VARIANTS: [&str; 1] = ["shuuro12"];
pub const DURATION_RANGE: [i64; 28] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30, 35, 40, 45, 60,
    75, 90,
];

pub struct WsState {
    pub players: Players,
    pub chat: ChatRooms,
    pub tx: broadcast::Sender<String>,
}

impl Default for WsState {
    fn default() -> Self {
        let players = Players::default();
        let chat = ChatRooms::default();
        let (tx, _rx) = broadcast::channel(100);
        Self { players, chat, tx }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct GameRequest {
    username: String,
    variant: String,
    time: i64,
    incr: i64,
    color: String,
}

impl GameRequest {
    /// Return true if game has valid time.
    pub fn is_valid(&self) -> bool {
        if VARIANTS.contains(&&self.variant[..]) {
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
    pub fn colors(&mut self, other: &String) -> [String; 2] {
        let mut c_s: [String; 2] = [String::from(""), String::from("")];
        let mut color = String::from(self.color());
        let other = String::from(other);
        let me = self.username();
        if color == "random" {
            color = self.random_color();
        }
        if color == "white" {
            c_s = [me, other];
        }
        // this is black
        else {
            c_s = [other, me];
        }
        c_s
    }

    pub fn color(&self) -> &String {
        &self.color
    }

    fn random_color(&self) -> String {
        if rand::random() {
            String::from("white")
        } else {
            String::from("black")
        }
    }
}
