use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_session::blake3::Hash;
use json_value_merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shuuro::{Color, Move};

use crate::{
    arc2,
    database::{mongo::ShuuroGame, redis::UserSession},
};

use super::GameGet;

use super::GameGet;

pub const VARIANTS: [&str; 1] = ["shuuro12"];
pub const DURATION_RANGE: [i64; 28] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30, 35, 40, 45, 60,
    75, 90,
];

#[derive(Clone, Serialize, Deserialize)]
pub struct GameRequest {
    pub username: String,
    variant: String,
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
    pub fn add(&self, mut game: GameRequest) -> Option<Value> {
        let mut all = self.all.lock().unwrap();
        if !all.contains_key(&game.username) {
            let res = game.response(&String::from("home_lobby_add"));
            all.insert(String::from(&game.username), game.clone());
            return Some(res);
        }
        None
    }

    pub fn remove(&self, t: &str, mut username: &String) -> Option<Value> {
        let mut all = self.all.lock().unwrap();
        if let Some(mut game) = all.remove(username) {
            let res = game.response(&String::from(t));
            return Some(res);
        }
        None
    }

    pub fn get_all(&self) -> Vec<GameRequest> {
        let all = self.all.lock().unwrap();
        let mut g = vec![];
        for i in all.values() {
            g.push(i.clone());
        }
        g
    }

    pub fn response(&self, all: Vec<GameRequest>) -> Value {
        json!({ "t": "home_lobby_full", "lobbyGames": all})
    }
}

pub struct ShuuroGames {
    all: Arc<Mutex<HashMap<String, ShuuroGame>>>,
}

impl Default for ShuuroGames {
    fn default() -> Self {
        Self {
            all: arc2(HashMap::new()),
        }
    }
}

impl ShuuroGames {
    pub fn add_game(&self, game: ShuuroGame) -> usize {
        let mut all = self.all.lock().unwrap();
        all.insert(String::from(&game._id), game);
        all.len()
    }

    pub fn remove_game(&self, id: &String) -> usize {
        let mut all = self.all.lock().unwrap();
        all.remove(id);
        all.len()
    }

    pub fn game_count(&self) -> usize {
        self.all.lock().unwrap().len()
    }

    pub fn get_hand(&self, id: &String, user: &UserSession) -> Option<String> {
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(id) {
            if let Some(index) = g.players.iter().position(|x| x == &user.username) {
                return Some(String::from(&g.hands[index]));
            }
        }
        println!("error he can't see hand");
        None
    }

    pub fn get_confirmed(&self, id: &String) -> Option<[bool; 2]> {
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(id) {
            let mut confirmed = [false, false];
            confirmed[0] = g.shuuro.0.is_confirmed(shuuro::Color::White);
            confirmed[1] = g.shuuro.0.is_confirmed(shuuro::Color::Black);
            return Some(confirmed);
        }
        None
    }

    pub fn get_game(&self, id: &String) -> Option<ShuuroGame> {
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(id) {
            return Some(g.clone());
        }
        None
    }

    pub fn buy(&self, json: &GameGet, player: &String) -> Option<String> {
        let all = self.all.lock().unwrap();
        if let Some(mut game) = all.get_mut(&json.game_id) {
            if let Some(p) = self.player_index(&game.players, player) {
                if let Some(m) = Move::from_sfen(&json.game_move) {
                    match m {
                        Move::Buy { piece } => {
                            if piece.color as usize == p {
                                game.shuuro.0.play(m);
                                if let Some(lm) =
                                    game.shuuro.0.get_sfen_history(&piece.color).last()
                                {
                                    return Some(String::from(&lm.0));
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
        None
    }

    fn player_index(&self, p: &[String; 2], u: &String) -> Option<usize> {
        p.iter().position(|x| x == u)
    }

    fn player_color(&self, index: usize) -> Color {
        if index == 0 {
            return Color::White;
        }
        Color::Black
    }
}
