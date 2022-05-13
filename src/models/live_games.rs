use actix::AsyncContext;
use actix::{Addr, Context};
use serde_json::Value;
use shuuro::{init, position::Outcome, Color, Move, PieceType, Position, Shop};

use crate::{models::model::ShuuroGame, websockets::lobby::Lobby};
use std::collections::{HashMap, HashSet};

use super::{
    db_work::start_clock,
    model::{TimeControl, TvGame},
};

#[derive(Clone)]
pub struct LiveGames {
    pub shuuro_games: HashMap<String, ShuuroLive>,
}

impl LiveGames {
    pub fn len(&self) -> usize {
        self.shuuro_games.len()
    }

    pub fn can_add(&self, username: &String) -> bool {
        for i in &self.shuuro_games {
            if i.1.can_add(&username) {
                return false;
            }
        }
        true
    }

    pub fn add_game(&mut self, id: &String, game: &ShuuroGame, ctx: &Context<Lobby>) {
        self.shuuro_games
            .insert(String::from(id), ShuuroLive::from(game));

        if let Some(mut g) = self.shuuro_games.get_mut(id) {
            g.add_spectator(game.white.as_str());
            g.add_spectator(game.black.as_str());

            if game.current_stage == 0 {
                let hand = format!("{}{}", &game.white_hand, &game.black_hand);
                g.shop.set_hand(&hand);
            } else if game.current_stage == 1 {
                g.deploy.set_sfen(&game.sfen);
                g.time_control.update_stage(1);
                g.game.side_to_move = g.deploy.side_to_move().to_string();
                g.game.last_clock = g.time_control.get_last_click();
            } else if game.current_stage == 2 {
                g.fight.set_sfen(&game.sfen);
                g.time_control.update_stage(2);
                g.game.last_clock = g.time_control.get_last_click();
            }
            start_clock(ctx.address(), id);
        }
    }

    pub fn remove_game(&mut self, id: &String) {
        self.shuuro_games.remove(id);
    }

    pub fn draw_req(&mut self, id: &String, username: &String) -> i8 {
        let game = self.shuuro_games.get_mut(id);
        if let Some(i) = game {
            let draw = i.game_draw(username);
            if !i.draws.contains(&false) {
                // both agree on draw
                i.game.status = 5;
                return 5;
            } else if draw {
                // one send draw request
                return -2;
            }
        }
        return -3;
    }

    pub fn resign(&mut self, id: &String, username: &String) -> bool {
        let game = self.shuuro_games.get_mut(id);
        if let Some(i) = game {
            let r = i.resign(username);
            return r;
        }
        return false;
    }

    pub fn get_game(&mut self, id: &String) -> Option<(String, ShuuroGame)> {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(i) => {
                return Some((String::from(id), i.game.clone()));
            }
            None => None,
        }
    }

    pub fn spectators(&self, id: &String) -> Option<&HashSet<String>> {
        if let Some(g) = self.shuuro_games.get(id) {
            return Some(g.spectators());
        }
        None
    }

    pub fn stop(&mut self, id: &String) {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(g) => {
                g.running = false;
            }
            None => {
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
            }
        }
    }

    pub fn place(&mut self, id: &String, game_move: String, username: &String) -> Option<Value> {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(g) => return g.place(game_move, username),
            None => {
            }
        }
        None
    }

    pub fn play(&mut self, id: &String, game_move: String, username: &String) -> Option<Value> {
        let game = self.shuuro_games.get_mut(id);
        match game {
            Some(g) => return g.play(game_move, username),
            None => {
            }
        }
        None
    }

    pub fn add_spectator(&mut self, id: &String, username: &str) -> usize {
        let game = self.shuuro_games.get_mut(id);
        if let Some(game) = game {
            return game.add_spectator(username);
        }
        0
    }

    pub fn remove_spectator(&mut self, id: &String, username: &str) -> usize {
        let game = self.shuuro_games.get_mut(id);
        if let Some(game) = game {
            return game.remove_spectator(username);
        }
        0
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
                    "path": format!("/shuuro/1/{}", game_id),
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

    pub fn get_all(&self) -> Vec<(String, ShuuroGame)> {
        let mut all = vec![];
        for i in self.shuuro_games.iter() {
            all.push((i.0.clone(), i.1.game.clone()));
        }
        all
    }

    pub fn set_all(&mut self, games: Vec<(String, ShuuroGame)>, ctx: &Context<Lobby>) {
        for i in games.iter() {
            self.add_game(&i.0, &i.1, ctx);
        }
    }

    pub fn get_tv(&self) -> Vec<TvGame> {
        let c = 0;
        let mut games = vec![];
        for i in &self.shuuro_games {
            if c == 20 {
                break;
            }
            let f = &i.1.game.sfen;
            if f == "" {
                continue;
            }
            let id = &i.1.game.game_id;
            let w = &i.1.game.white;
            let b = &i.1.game.black;
            let t = "live_tv";
            let tv = TvGame::new(t, id, w, b, f);
            games.push(tv);
        }
        games
    }

    pub fn time_ok(&self, game_id: &str) -> Option<bool> {
        if let Some(g) = self.shuuro_games.get(game_id) {
            return Some(g.time_ok());
        }
        None 
    }
    pub fn lost_on_time(&mut self, game_id: &String) -> Option<&ShuuroGame> {
        let game = self.shuuro_games.get_mut(game_id);
        if let Some(mut i) = game {
            let stm = &i.game.side_to_move.clone();
            i.lost_on_time(stm);
            return Some(&i.game);
        }
        None
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
    pub draws: [bool; 2],
    pub spectators: HashSet<String>,
}

impl From<&ShuuroGame> for ShuuroLive {
    fn from(game: &ShuuroGame) -> Self {
        let time_control = TimeControl::from(game);
        ShuuroLive {
            game: game.clone(),
            shop: Shop::default(),
            deploy: Position::default(),
            fight: Position::default(),
            running: true,
            time_control, 
            draws: [false, false],
            spectators: HashSet::new(),
        }
    }
}

impl ShuuroLive {
    pub fn can_add(&self, username: &String) -> bool {
        if &self.game.white == username || &self.game.black == username {
            return false;
        }
        true
    }

    pub fn players(&self) -> [&String; 2] {
        [&self.game.white, &self.game.black]
    }

    pub fn time_ok(&self) -> bool {
        self.time_control.time_ok(&self.game.side_to_move)
    }

    pub fn spectators(&self) -> &HashSet<String> {
        &self.spectators
    }

    pub fn add_spectator(&mut self, username: &str) -> usize {
        self.spectators.insert(String::from(username));
        self.spectators.len()
    }

    pub fn remove_spectator(&mut self, username: &str) -> usize {
        self.spectators.remove(&String::from(username));
        self.spectators.len()
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
        if self.game.current_stage == 1 {
            return format!(
                "{}{}",
                &self.deploy.get_hand(Color::Black),
                &self.deploy.get_hand(Color::White)
            );
        }

        if *color == Color::NoColor && self.game.current_stage == 0 {
            return String::from("");
        }
        if self.game.current_stage == 0 || self.game.current_stage == 1 {
            return self.shop.to_sfen(*color);
        }
        return String::from("");
    }

    pub fn buy(&mut self, game_move: String, username: &String) {
        if &self.game.current_stage == &0 {
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
                                self.draws = [false, false];
                                if last_credit != self.shop.credit(*color) {
                                    let hand = self.get_hand(&username);
                                    self.set_hand(color, &hand);
                                    /*
                                    self.game.shop_history.push(
                                        self.shop.get_sfen_history(color).last().unwrap().clone(),
                                    );
                                    */
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
                            self.game.side_to_move = String::from("b");
                        } else {
                            self.game.black_clock =
                                self.time_control.get_clock(c.chars().last().unwrap());
                            self.game.side_to_move = String::from("w");
                        }
                    }
                }
            }
        }
    }

    pub fn set_hand(&mut self, color: &Color, hand: &String) {
        match color {
            Color::White => self.game.white_hand = String::from(hand),
            Color::Black => self.game.black_hand = String::from(hand),
            _ => (),
        }
    }

    pub fn game_draw(&mut self, username: &String) -> bool {
        let player_color = self.player_color(username);
        if player_color == Color::NoColor {
            return false;
        } else if self.draws[player_color.index()] {
            return false;
        }
        self.draws[player_color.index()] = true;
        true
    }

    pub fn set_deploy(&mut self) {
        self.game.current_stage = 1;
        self.time_control.update_stage(1);
        self.load_shop_hand();
        self.deploy.generate_plinths();
        self.game.sfen = self.deploy.to_sfen();
    }

    pub fn set_fight(&mut self, color: Color) -> bool {
        self.game.current_stage = 2;
        self.time_control.update_stage(2);
        self.time_control.click(color);
        self.game.last_clock = self.time_control.get_last_click();
        let sfen = self.deploy.generate_sfen();
        self.fight.set_sfen(&sfen.as_str());
        self.fight.in_check(self.fight.side_to_move().flip())
    }

    pub fn place(&mut self, game_move: String, username: &String) -> Option<Value> {
        if self.game.current_stage == 1 {
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
                                self.draws = [false, false];
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
        if self.game.current_stage == 2 {
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
                                        self.draws = [false, false];
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

    pub fn resign(&mut self, username: &String) -> bool {
        let players = self.players();
        if players.contains(&username) {
            let color = self.player_color(username);
            self.game.status = 7;
            self.game.result = color.to_string();
            self.time_control.click(color);
            self.game.last_clock = self.time_control.get_last_click();
            return true;
        }
        false
    }

    pub fn lost_on_time(&mut self, stm: &String) -> &ShuuroGame {
        self.game.result = String::from(stm);
        if stm == "" {
            self.game.status = 5;
        } else {
            self.game.status = 8;
        }
        &self.game
    }

    fn player_color(&self, username: &String) -> Color {
        if username == &self.game.white {
            return Color::White;
        } else if username == &self.game.black {
            return Color::Black;
        }
        Color::NoColor
    }
}
