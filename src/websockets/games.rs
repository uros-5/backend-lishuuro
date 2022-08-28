use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration, NaiveTime, Utc};
use json_value_merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shuuro::{init, Color, Move};

use crate::{
    arc2,
    database::{mongo::ShuuroGame, redis::UserSession},
};

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
        let c_s: [String; 2];
        let mut color = String::from("");
        if &self.color == "random" {
            color = self.random_color();
        }
        if color == "white" {
            c_s = [String::from(&self.username), String::from(other)];
        }
        // this is black
        else {
            c_s = [String::from(other), String::from(&self.username)];
        }
        c_s
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
            if game.is_valid() {
                let res = game.response(&String::from("home_lobby_add"));
                all.insert(String::from(&game.username), game.clone());
                return Some(res);
            }
        }
        None
    }

    pub fn remove(&self, t: &str, username: &String) -> Option<Value> {
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
        init();
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

    pub fn _remove_game(&self, id: &String) -> usize {
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
                let color = Color::from(index);
                return Some(g.shuuro.0.to_sfen(color));
            }
        }
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

    pub fn buy(&self, json: &GameGet, player: &String) -> Option<[bool; 2]> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            if let Some(p) = self.player_index(&game.players, player) {
                if let Some(m) = Move::from_sfen(&json.game_move) {
                    match m {
                        Move::Buy { piece } => {
                            let color = Color::from(p);
                            if color == piece.color {
                                if let Some(confirmed) = game.shuuro.0.play(m) {
                                    if confirmed[color as usize] == true {
                                        return Some(confirmed);
                                    }
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

    pub fn place_move(&self, json: &GameGet, player: &String) -> Option<(String, [String; 2])> {
        let mut all = self.all.lock().unwrap();
        if let Some(mut game) = all.get_mut(&json.game_id) {
            if let Some(index) = self.player_index(&game.players, player) {
                if index == game.side_to_move as usize {
                    if let Some(gm) = Move::from_sfen(&json.game_move.as_str()) {
                        match gm {
                            Move::Put { to, piece } => {
                                if Color::from(index) == piece.color {
                                    if let Some(s) = game.shuuro.1.place(piece, to) {
                                        if game.tc.click(index) {
                                            return None;
                                        }
                                        return Some((s, game.players.clone()));
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
        None
    }

    pub fn set_deploy(&self, id: &String) -> Option<Value> {
        if let Some(game) = self.all.lock().unwrap().get_mut(id) {
            game.current_stage = 1;
            let hand = self.load_shop_hand(game);
            game.shuuro.1.generate_plinths();
            game.sfen = game.shuuro.1.to_sfen();
            game.side_to_move = 0;
            let value = serde_json::json!({
                "t": "redirect_deploy",
                "path": format!("/shuuro/1/{}", id),
                "hand": hand,
                "last_clock": Utc::now(),
                "side_to_move": "w",
                "w": String::from(&game.players[0]),
                "b": String::from(&game.players[1]),
                "sfen": game.sfen
            });
            return Some(value);
        }
        None
    }

    pub fn load_shop_hand(&self, game: &mut ShuuroGame) -> String {
        let w = game.shuuro.0.to_sfen(Color::White);
        let b = game.shuuro.0.to_sfen(Color::Black);
        game.hands = [w.clone(), b.clone()];
        let hand = format!("{}{}", w, b);
        let sfen = "57/57/57/57/57/57/57/57/57/57/57/57 w";
        game.shuuro.1.set_hand(hand.as_str());
        game.shuuro.1.set_sfen(&sfen);
        hand
    }

    pub fn get_players(&self, id: &String) -> Option<[String; 2]> {
        if let Some(game) = self.all.lock().unwrap().get(id) {
            return Some(game.players.clone());
        }
        None
    }

    pub fn confirm(&self, id: &String, user: &UserSession) -> Option<[bool; 2]> {
        if let Some(game) = self.all.lock().unwrap().get_mut(id) {
            if let Some(p) = self.player_index(&game.players, &user.username) {
                let c = Color::from(p);
                game.shuuro.0.confirm(c);
                let mut confirmed = [false, false];
                confirmed[0] = game.shuuro.0.is_confirmed(Color::White);
                confirmed[1] = game.shuuro.0.is_confirmed(Color::Black);
                return Some(confirmed);
            }
        }
        None
    }

    fn player_index(&self, p: &[String; 2], u: &String) -> Option<usize> {
        p.iter().position(|x| x == u)
    }
}

#[derive(Debug, Clone)]
pub struct TimeControl {
    pub last_click: NaiveTime,
    pub clocks: [Duration; 2],
    pub stage: u8,
    pub incr: Duration,
}

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::new(10, 5)
    }
}

impl TimeControl {
    pub fn new(time: i64, incr: i64) -> Self {
        let duration = Duration::seconds(time * 60 + incr);
        let last_click = Utc::now().time();
        Self {
            clocks: [duration, duration.clone()],
            stage: 0,
            incr: Duration::seconds(incr),
            last_click,
        }
    }

    pub fn update_stage(&mut self, stage: u8) {
        self.stage = stage;
        self.last_click = Utc::now().time();
    }

    pub fn click(&mut self, color: usize) -> bool {
        let elapsed = self.elapsed();
        if let Some(duration) = self.clocks[color].checked_sub(&elapsed) {
            if duration.is_zero() {
                return false;
            }
            self.clocks[color] = duration;
            let duration = duration.checked_add(&self.incr).unwrap();
            self.clocks[color] = self.incr;
            return true;
        }
        false
    }

    fn elapsed(&self) -> Duration {
        Utc::now().time() - self.last_click
    }
}
