use serde_json::Value;
use shuuro::{Color, Move, Piece, Position, Shop};

use crate::models::model::ShuuroGame;
use std::collections::HashMap;

use super::model::TimeControl;

#[derive(Clone)]
pub struct LiveGames {
    pub shuuro_games: HashMap<String, ShuuroLive>,
}

impl LiveGames {
    pub fn can_add(&self, username: &String) -> bool {
        for i in &self.shuuro_games {
            if i.1.can_add(&username) {
                return false;
            }
        }
        true
    }

    pub fn add_game(&mut self, id: String, game: &ShuuroGame) {
        self.shuuro_games.insert(id, ShuuroLive::from(game));
    }

    pub fn get_game(&mut self, id: String) -> Option<(String, ShuuroGame)> {
        let game = self.shuuro_games.get_mut(&id);
        match game {
            Some(i) => {
                i.format_res();
                return Some((String::from(id), i.game.clone()));
            }
            None => None,
        }
    }

    pub fn stop(&mut self, id: String) {
        let game = self.shuuro_games.get_mut(&id);
        match game {
            Some(g) => {
                g.running = false;
            }
            None => {
                println!("game not found");
            }
        }
    }

    // another parameter username
    pub fn buy(&mut self, id: &String, game_move: String, username: &String) {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(g) => {
                g.buy(game_move, username);
            }
            None => {
                println!("game not found")
            }
        }
    }

    pub fn players(&self, game_id: &String) -> [String; 2] {
        let game = self.shuuro_games.get(game_id);
        if let Some(g) = game {
            return g.players();
        }
        [String::from(""), String::from("")]
    }

    pub fn confirmed_players(&self, game_id: &String) -> [bool; 2] {
        let game = self.shuuro_games.get(game_id);
        if let Some(g) = game {
            return g.confirmed_players();
        }
        [false, false]
    }

    pub fn get_hand(&self, game_id: String, username: &String) -> String {
        let game = self.shuuro_games.get(&game_id);

        match game {
            Some(g) => {
                return g.get_hand(username);
            }
            None => (),
        }
        String::from("")
    }

    pub fn set_deploy(&mut self, game_id: &String) -> Value {
        let game = self.shuuro_games.get_mut(game_id);
        match game {
            Some(g) => {
                g.set_deploy();
                g.game.side_to_move = g.deploy.side_to_move().to_string();
                g.game.last_clock = g.time_control.get_last_click();
                return serde_json::json!({"t": "redirect_deploy",
                    "path": format!("/shuuro/deploy/{}", game_id),
                    "hand":g.get_hand(&String::from("")),
                    "last_clock": g.time_control.get_last_click().to_string(),
                    "side_to_move": "white",
                    "sfen": g.game.sfen});
            }
            None => {
                return serde_json::json!({"t": "error"});
            }
        }
    }
}
impl Default for LiveGames {
    fn default() -> Self {
        LiveGames {
            shuuro_games: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct ShuuroLive {
    pub game: ShuuroGame,
    pub shop: Shop,
    pub deploy: Position,
    pub fight: Position,
    pub running: bool,
    pub time_control: TimeControl,
}

impl From<&ShuuroGame> for ShuuroLive {
    fn from(game: &ShuuroGame) -> Self {
        ShuuroLive {
            game: game.clone(),
            shop: Shop::default(),
            deploy: Position::default(),
            fight: Position::default(),
            running: true,
            time_control: TimeControl::new(game.incr.whole_seconds(), game.min.whole_seconds()),
        }
    }
}

impl ShuuroLive {
    pub fn format_res(&mut self) {
        self.game.last_clock = self.time_control.get_last_click();
        self.game.status = -2;
        self.game.result = String::from("*");
    }

    pub fn can_add(&self, username: &String) -> bool {
        if &self.game.white == username || &self.game.black == username {
            return false;
        }
        true
    }

    pub fn players(&self) -> [String; 2] {
        [self.game.white.clone(), self.game.black.clone()]
    }

    pub fn confirmed_players(&self) -> [bool; 2] {
        [
            self.shop.is_confirmed(Color::White),
            self.shop.is_confirmed(Color::Black),
        ]
    }

    pub fn load_shop_hand(&mut self) {
        let w = self.shop.to_sfen(Color::Black);
        let b = self.shop.to_sfen(Color::White);
        self.deploy.set_hand(format!("{}{}", w, b).as_str());
    }

    pub fn get_hand(&self, username: &String) -> String {
        let color = &self.game.user_color(username);
        if self.game.current_stage == "deploy" {
            return format!(
                "{}{}",
                &self.deploy.get_hand(Color::Black),
                &self.deploy.get_hand(Color::White)
            );
        }

        if *color == Color::NoColor && self.game.current_stage == "shop" {
            return String::from("");
        }
        if self.game.current_stage == "shop" || self.game.current_stage == "deploy" {
            return self.shop.to_sfen(*color);
        }
        return String::from("");
    }

    pub fn buy(&mut self, game_move: String, username: &String) {
        if &self.game.current_stage == &String::from("shop") {
            if &game_move.len() == &2 {
                let color = &self.game.user_color(username);
                if *color == Color::NoColor {
                    return ();
                }
                let piece = game_move.chars().last();
                if let Some(s) = piece {
                    let piece = Piece::from_sfen(s);
                    if let Some(p) = piece {
                        let m = Move::Buy { piece: p };
                        if p.color != *color {
                            return ();
                        }
                        let c = color.to_string();
                        if self.time_control.time_ok(&c) {
                            self.shop.play(m);
                            self.game
                                .shop_history
                                .push(self.shop.get_sfen_history(color).last().unwrap().clone());
                        }
                        match color {
                            Color::White => {
                                self.game.white_credit = self.shop.credit(*color) as u16;
                            }
                            Color::Black => {
                                self.game.black_credit = self.shop.credit(*color) as u16;
                            }
                            _ => (),
                        }
                    } else {
                        let c = color.to_string();
                        if self.time_control.time_ok(&c) {
                            self.shop.confirm(*color);
                            self.time_control.click(*color);
                            if c == "w" {
                                self.game.white_clock =
                                    self.time_control.get_clock(c.chars().last().unwrap());
                            } else {
                                self.game.black_clock =
                                    self.time_control.get_clock(c.chars().last().unwrap());
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_deploy(&mut self) {
        self.game.current_stage = String::from("deploy");
        self.time_control.update_stage(String::from("deploy"));
        self.load_shop_hand();
        self.deploy.generate_plinths();
        self.game.sfen = self.deploy.to_sfen();
        
    }
}
