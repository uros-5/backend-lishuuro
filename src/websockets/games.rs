use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bson::DateTime as DT;
use chrono::Utc;
use json_value_merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shuuro::{init, position::Outcome, Color, Move, PieceType, Position};

use crate::{
    arc2,
    database::{
        mongo::{array_i32_duration, duration_i32_array, ShuuroGame},
        redis::UserSession,
    },
};

use super::{GameGet, LiveGameMove};

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
    /// Add new game to live games.
    pub fn add_game(&self, game: ShuuroGame) -> usize {
        let mut all = self.all.lock().unwrap();
        all.insert(String::from(&game._id), game);
        all.len()
    }

    /// Remove game after end.
    pub fn _remove_game(&self, id: &String) -> usize {
        let mut all = self.all.lock().unwrap();
        all.remove(id);
        all.len()
    }

    /// Count all games.
    pub fn game_count(&self) -> usize {
        self.all.lock().unwrap().len()
    }

    // SHOP PART

    /// Get hand for active player.
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

    /// Get confirmed players.
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

    /// Buy one piece.
    pub fn buy(&self, json: &GameGet, player: &String) -> Option<LiveGameMove> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            if let Some(p) = self.player_index(&game.players, player) {
                if let Some(c) = game.tc.click(p) {
                    if let Some(m) = Move::from_sfen(&json.game_move) {
                        match m {
                            Move::Buy { piece } => {
                                let color = Color::from(p);
                                if color == piece.color {
                                    if let Some(confirmed) = game.shuuro.0.play(m) {
                                        if confirmed[color as usize] == true {
                                            return Some(LiveGameMove::BuyMove(confirmed));
                                        }
                                    }
                                }
                            }
                            _ => (),
                        }
                    } else {
                        game.shuuro.0.confirm(Color::from(p));
                        let confirmed = [
                            game.shuuro.0.is_confirmed(Color::White),
                            game.shuuro.0.is_confirmed(Color::Black),
                        ];
                        return Some(LiveGameMove::BuyMove(confirmed));
                    }
                } else {
                    return Some(LiveGameMove::LostOnTime(p));
                }
            }
        }
        None
    }

    /// Transfer hand from shop to deploy part.
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

    /// Check if deployment is over by checking player's hands.
    pub fn is_deployment_over(&self, g: &ShuuroGame) -> bool {
        let mut completed: [bool; 3] = [false, false, false];
        let color_iter = Color::iter();
        for i in color_iter {
            completed[i.index()] = g.shuuro.1.is_hand_empty(&i, PieceType::Plinth);
        }
        completed[2] = true;
        !completed.contains(&false)
    }

    // DEPLOY PART

    /// Place piece on board.
    pub fn place_move(&self, json: &GameGet, player: &String) -> Option<LiveGameMove> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            if game.current_stage != 1 {
                return None;
            }
            if let Some(index) = self.player_index(&game.players, player) {
                if game.shuuro.1.side_to_move() != Color::from(index) {
                    return None;
                }
                if let Some(clocks) = game.tc.click(index) {
                    game.clocks = game.tc.clocks;
                    game.last_clock = DT::now();
                    if let Some(gm) = Move::from_sfen(&json.game_move.as_str()) {
                        match gm {
                            Move::Put { to, piece } => {
                                if Color::from(index) == piece.color {
                                    if let Some(s) = game.shuuro.1.place(piece, to) {
                                        let mut fme = false;
                                        let tf = self.is_deployment_over(&game);
                                        if tf {
                                            fme = self.set_fight(game);
                                        }
                                        game.side_to_move = self
                                            .another_index(game.shuuro.1.side_to_move() as usize)
                                            as u8;
                                        game.sfen = game.shuuro.1.generate_sfen();
                                        game.hands = [
                                            game.shuuro.1.get_hand(Color::White),
                                            game.shuuro.1.get_hand(Color::Black),
                                        ];
                                        game.history.1.push((s.clone(), 1));
                                        return Some(LiveGameMove::PlaceMove(
                                            s.split("_").next().unwrap().to_string(),
                                            clocks,
                                            fme,
                                            tf,
                                            game.players.clone(),
                                        ));
                                    }
                                } else {
                                    println!("wrong piece, winner {}", self.another_index(index));
                                }
                            }
                            _ => (),
                        }
                    }
                } else {
                    println!("{} lost on time", index);
                }
            }
        }
        None
    }

    /// Transfer data from deployment to fighting part of game.
    pub fn set_fight(&self, game: &mut ShuuroGame) -> bool {
        game.current_stage = 2;
        game.tc.update_stage(2);
        game.last_clock = DT::now();
        let sfen = game.shuuro.1.generate_sfen();
        game.shuuro.2.set_sfen(&sfen.as_str());
        game.shuuro.2.in_check(game.shuuro.2.side_to_move().flip())
    }

    pub fn fight_move(&self, json: &GameGet, player: &String) -> Option<LiveGameMove> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            if game.current_stage != 2 {
                return None;
            }
            if let Some(index) = self.player_index(&game.players, player) {
                if game.shuuro.2.side_to_move() != Color::from(index) {
                    return None;
                }
                if let Some(clocks) = game.tc.click(index) {
                    game.clocks = game.tc.clocks;
                    game.last_clock = DT::now();
                    if let Some(gm) = Move::from_sfen(&json.game_move.as_str()) {
                        match gm {
                            Move::Normal { from, to, promote } => {
                                if let Some(piece) = game.shuuro.2.piece_at(from) {
                                    if Color::from(index) == piece.color {
                                        if let Ok(_) = game.shuuro.2.play(
                                            from.to_string().as_str(),
                                            to.to_string().as_str(),
                                        ) {
                                            let outcome = self.update_status(game);
                                            let stm = self.another_index(game.shuuro.2.side_to_move() as usize); 
                                            game.side_to_move = stm as u8; 
                                            game.sfen = game.shuuro.2.generate_sfen();
                                            let m = game.shuuro.2.get_sfen_history().last().unwrap();
                                            game.history.2.push(m.clone());
                                            return Some(LiveGameMove::FightMove(
                                                String::from(&json.game_move),
                                                clocks,
                                                game.status,
                                                game.result.clone(),
                                                game.players.clone(),
                                                outcome
                                            ));
                                        }
                                    }
                                } else {
                                    println!("wrong piece, winner {}", self.another_index(index));
                                }
                            }
                            _ => (),
                        }
                    }
                } else {
                    println!("{} lost on time", index);
                }
            }
        }
        None
    }

    pub fn update_status(&self, game: &mut ShuuroGame) -> String  {
        let outcome = game.shuuro.2.outcome();
        match outcome {
            Outcome::Check { color: _ } => {
                game.status = -1;
            }
            Outcome::Nothing => {
                game.status = -1;
            }
            Outcome::Stalemate => {
                game.status = 3;
            }
            Outcome::Draw => {
                game.status = 5;
            }
            Outcome::DrawByRepetition => game.status = 4,
            Outcome::DrawByMaterial => game.status = 6,
            Outcome::Checkmate { color } => {
                game.status = 1;
                game.result = color.to_string();
            }
            Outcome::MoveOk => {
                game.status = -1;
            }
            Outcome::MoveNotOk => {
                game.status = -2;
            }
        }
        outcome.to_string()
        
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

    // HELPER METHODS

    pub fn get_players(&self, id: &String) -> Option<[String; 2]> {
        if let Some(game) = self.all.lock().unwrap().get(id) {
            return Some(game.players.clone());
        }
        None
    }

    fn player_index(&self, p: &[String; 2], u: &String) -> Option<usize> {
        p.iter().position(|x| x == u)
    }

    fn another_index(&self, index: usize) -> usize {
        if index == 0 {
            return 1;
        }
        0
    }

    /// Get live or archived game(if it exist).
    pub fn get_game(&self, id: &String) -> Option<ShuuroGame> {
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(id) {
            return Some(g.clone());
        }
        None
    }
}
