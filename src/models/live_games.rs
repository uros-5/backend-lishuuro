use serde_json::Value;
use shuuro::{init, position::Outcome, Color, Move, PieceType, Position, Shop};

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

    pub fn remove_game(&mut self, id: &String) {
        self.shuuro_games.remove(id);
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

    pub fn place(&mut self, id: &String, game_move: String, username: &String) -> Option<Value> {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(g) => return g.place(game_move, username),
            None => {
                println!("game not found")
            }
        }
        None
    }

    pub fn play(&mut self, id: &String, game_move: String, username: &String) -> Option<Value> {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(g) => return g.play(game_move, username),
            None => {
                println!("game not found")
            }
        }
        None
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
                    "side_to_move": "w",
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
        init();
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
        let hand = format!("{}{}", w, b);
        let sfen = "57/57/57/57/57/57/57/57/57/57/57/57 w";
        self.deploy.set_hand(hand.as_str());
        self.deploy.set_sfen(&sfen);
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
                let m = Move::from_sfen(&game_move.as_str());
                if let Some(m) = m {
                    match m {
                        Move::Buy { piece } => {
                            if piece.color != *color {
                                return ();
                            }
                            let c = color.to_string();
                            let last_credit = self.shop.credit(*color);
                            if self.time_control.time_ok(&c) {
                                self.shop.play(m);
                                if last_credit != self.shop.credit(*color) {
                                    self.game.shop_history.push(
                                        self.shop.get_sfen_history(color).last().unwrap().clone(),
                                    );
                                }
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

    pub fn set_deploy(&mut self) {
        self.game.current_stage = String::from("deploy");
        self.time_control.update_stage(String::from("deploy"));
        self.load_shop_hand();
        self.deploy.generate_plinths();
        self.game.sfen = self.deploy.to_sfen();
    }

    pub fn set_fight(&mut self, color: Color) -> bool {
        self.game.current_stage = String::from("fight");
        self.time_control.update_stage(String::from("fight"));
        self.time_control.click(color);
        self.game.last_clock = self.time_control.get_last_click();
        let sfen = self.deploy.generate_sfen();
        self.fight.set_sfen(&sfen.as_str());
        self.fight.in_check(self.fight.side_to_move().flip())
    }

    pub fn place(&mut self, game_move: String, username: &String) -> Option<Value> {
        if self.game.current_stage == "deploy" {
            if let Some(m) = Move::from_sfen(game_move.as_str()) {
                match m {
                    Move::Put { to, piece } => {
                        let color = self.game.user_color(username);
                        let ply = self.deploy.ply();
                        if color == Color::NoColor {
                            return None;
                        } else if piece.color != color {
                            return None;
                        } else if self.time_control.time_ok(&color.to_string()) {
                            if self.game.side_to_move == color.to_string() {
                                self.deploy.place(piece, to);
                                let ply_2 = self.deploy.ply();
                                if ply_2 != ply {
                                    let m = self.deploy.get_sfen_history().last().unwrap().clone();
                                    self.time_control.click(color);
                                    self.game.side_to_move = self.deploy.side_to_move().to_string();
                                    self.game.sfen = self.deploy.generate_sfen();
                                    self.game.last_clock = self.time_control.get_last_click();
                                    self.game.black_clock = self.time_control.get_clock('b');
                                    self.game.white_clock = self.time_control.get_clock('w');
                                    self.game.deploy_history.push(m);
                                    let to_fight = self.is_deployment_over();
                                    let mut first_move_error = false;
                                    if to_fight {
                                        self.set_fight(color);
                                        first_move_error = self.set_fight(color);
                                        if first_move_error {
                                            self.game.status = 7;
                                        }
                                    }
                                    return Some(serde_json::json!({"t": "live_game_place",
                                            "move": game_move, 
                                            "game_id": "",
                                            "to_fight": self.is_deployment_over(),
                                            "first_move_error": first_move_error }));
                                }
                            }
                        }
                        ()
                    }
                    _ => (),
                }
            }
        }
        return None;
    }

    pub fn play(&mut self, game_move: String, username: &String) -> Option<Value> {
        if self.game.current_stage == "fight" {
            if let Some(m) = Move::from_sfen(game_move.as_str()) {
                match m {
                    Move::Normal {
                        from,
                        to,
                        promote: _,
                    } => {
                        let color = self.game.user_color(username);
                        if color == Color::NoColor {
                            return None;
                        } else if self.time_control.time_ok(&color.to_string()) {
                            if let Some(piece) = self.fight.piece_at(from) {
                                if piece.color == color {
                                    if let Ok(m) = self
                                        .fight
                                        .play(from.to_string().as_str(), to.to_string().as_str())
                                    {
                                        let mut res = serde_json::json!({"t": "live_game_play",
                                        "game_move": game_move,
                                        "status": 0 as i64,
                                        "game_id": "", "outcome": m.to_string()});
                                        self.game.side_to_move =
                                            self.fight.side_to_move().to_string();
                                        self.game.sfen = self.fight.generate_sfen();
                                        self.update_status();
                                        *res.get_mut("status").unwrap() =
                                            serde_json::json!(self.game.status as i64);
                                        let m =
                                            self.fight.get_sfen_history().last().unwrap().clone();
                                        self.game.fight_history.push(m);
                                        self.time_control.click(color);
                                        self.game.last_clock = self.time_control.get_last_click();
                                        self.game.black_clock = self.time_control.get_clock('b');
                                        self.game.white_clock = self.time_control.get_clock('w');
                                        return Some(res);
                                    } else {
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        None
    }

    pub fn is_deployment_over(&self) -> bool {
        let mut completed: [bool; 3] = [false, false, false];
        let color_iter = Color::iter();
        for i in color_iter {
            completed[i.index()] = self.deploy.is_hand_empty(&i, PieceType::Plinth);
        }
        completed[2] = true;
        !completed.contains(&false)
    }

    pub fn update_status(&mut self) {
        match self.fight.outcome() {
            Outcome::Check { color: _ } => {
                self.game.status = -1;
            }
            Outcome::Nothing => {
                self.game.status = -1;
            }
            Outcome::Stalemate => {
                self.game.status = 3;
            }
            Outcome::Draw => {
                self.game.status = 5;
            }
            Outcome::DrawByRepetition => self.game.status = 4,
            Outcome::DrawByMaterial => self.game.status = 6,
            Outcome::Checkmate { color } => {
                self.game.status = 1;
                self.game.result = color.to_string();
            }
            Outcome::MoveOk => {
                self.game.status = -1;
            }
            Outcome::MoveNotOk => {
                self.game.status = -2;
            }
        }
    }
}
