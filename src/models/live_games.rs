use shuuro::{Color, Move, Piece, Position, Shop};

use crate::models::model::ShuuroGame;
use std::{collections::HashMap, thread, time::Duration};

use super::model::TimeControl;

#[derive(Clone)]
pub struct LiveGames {
    pub shuuro_games: HashMap<String, ShuuroLive>,
}

impl LiveGames {
    pub fn can_add(&self, username: &String) -> bool {
        for i in &self.shuuro_games {
            if &i.1.game.white == username || &i.1.game.black == username {
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
                if &g.game.current_stage == &String::from("shop") {
                    if &game_move.len() == &2 {
                        let color = &g.game.user_color(username);
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
                                let c = color.to_string().chars().nth(0).unwrap();
                                if g.time_control.time_ok(c) {
                                    g.shop.play(m);
                                }
                                match color {
                                    Color::White => {
                                        g.game.white_credit = g.shop.credit(*color) as u16;
                                    }
                                    Color::Black => {
                                        g.game.black_credit = g.shop.credit(*color) as u16;
                                    }
                                    _ => (),
                                }
                            } else {
                                let c = color.to_string().chars().nth(0).unwrap();
                                if g.time_control.time_ok(c) {
                                    g.shop.confirm(*color);
                                }
                            }
                        }
                    }
                }
            }
            None => {
                println!("game not found")
            }
        }
    }

    pub fn players(&self, game_id: &String) -> [String; 2] {
        let game = self.shuuro_games.get(game_id);
        if let Some(g) = game {
            let players = [g.game.white.clone(), g.game.black.clone()];
            return players;
        }
        [String::from(""), String::from("")]
    }

    pub fn is_shop_done(&self, game_id: &String) -> bool {
        let game = self.shuuro_games.get(game_id);
        if let Some(g) = game {
            let players = [
                g.shop.is_confirmed(Color::White),
                g.shop.is_confirmed(Color::Black),
            ];
            println!("{:?}", &players);
            return !players.contains(&false);
        }
        false
    }

    pub fn get_hand(&self, game_id: String, username: &String) -> String {
        let game = self.shuuro_games.get(&game_id);

        match game {
            Some(g) => {
                let color = &g.game.user_color(username);
                if *color == Color::NoColor && g.game.current_stage == "shop" {
                    return String::from("");
                }
                if g.game.current_stage == "shop" || g.game.current_stage == "deploy" {
                    return g.shop.to_sfen(*color);
                }
            }
            None => (),
        }
        String::from("")
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
        self.game.shop_history = self.shop.get_sfen_history(&Color::NoColor).clone();
        self.game.deploy_history = self.deploy.get_sfen_history().clone();
        self.game.fight_history = self.fight.get_sfen_history().clone();
        self.game.white_clock = self.time_control.get_clock('w');
        self.game.black_clock = self.time_control.get_clock('b');
        self.game.last_clock = self.time_control.get_last_click();
        self.game.status = -2;
        self.game.result = String::from("*");
    }
}
