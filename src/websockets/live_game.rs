use std::{
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    ops::{BitAnd, BitOr, BitOrAssign, Not},
    sync::{Arc, Mutex, MutexGuard},
};

use bson::DateTime as DT;
use chrono::Utc;
use mongodb::Collection;
use serde_json::Value;
use shuuro::{
    attacks::Attacks,
    bitboard::BitBoard,
    position::{Board, Outcome, Position, Sfen},
    shuuro12::{
        attacks12::Attacks12, bitboard12::BB12, position12::P12,
        square12::Square12,
    },
    shuuro8::{
        attacks8::Attacks8, bitboard8::BB8, position8::P8, square8::Square8,
    },
    Color, Move, MoveError, MoveRecord, Piece, PieceType, SfenError, Shop,
    Square, Variant,
};

use crate::{
    arc2,
    database::{
        mongo::ShuuroGame, queries::update_entire_game, redis::UserSession,
    },
};

use super::{
    time_control::TimeCheck, GameGet, LiveGameMove, MessageHandler,
    MsgDatabase, TvGame,
};

macro_rules! send {
    (0, $self: ident, $json: expr, $method: ident, $($params:expr),*) => {
        if $json.variant.contains("12") {
            $self.live_games12.$method($($params),*)
        } else {
            $self.live_games8.$method($($params),*)
        }
    };

    (1, $self: ident, $json: expr, $method: ident, $($params:expr),*) => {
        if $json.variant.contains("12") {
            $self.live_games12.$method($($params),*).await
        } else {
            $self.live_games8.$method($($params),*).await
        }
    };
}

#[derive(Debug, Clone)]
pub struct LiveGame<S, B, A, P>
where
    S: Square + Hash,
    B: BitBoard<S>,
    A: Attacks<S, B>,
    P: Position<S, B, A>,
    Self: Sized,
    for<'a> &'a B: BitOr<&'a B, Output = B>,
    for<'a> &'a B: BitAnd<&'a B, Output = B>,
    for<'a> &'a B: Not<Output = B>,
    for<'a> &'a B: BitOr<&'a S, Output = B>,
    for<'a> &'a B: BitAnd<&'a S, Output = B>,
    for<'a> B: BitOrAssign<&'a S>,
{
    shop: Shop<S>,
    placement: P,
    fight: P,
    pub game: ShuuroGame,
    _b: PhantomData<B>,
    _a: PhantomData<A>,
}

impl<S, B, A, P> LiveGame<S, B, A, P>
where
    S: Square + Hash,
    B: BitBoard<S>,
    A: Attacks<S, B>,
    P: Position<S, B, A>,
    Self: Sized,
    for<'a> &'a B: BitOr<&'a B, Output = B>,
    for<'a> &'a B: BitAnd<&'a B, Output = B>,
    for<'a> &'a B: Not<Output = B>,
    for<'a> &'a B: BitOr<&'a S, Output = B>,
    for<'a> &'a B: BitAnd<&'a S, Output = B>,
    for<'a> B: BitOrAssign<&'a S>,
{
    fn new(game: ShuuroGame) -> Self {
        Self {
            _b: PhantomData,
            _a: PhantomData,
            shop: Shop::<S>::default(),
            placement: P::new(),
            fight: P::new(),
            game,
        }
    }

    // SHOP PART

    /// Change variant from the start of the game.
    pub fn change_variant(&mut self, variant: &String) {
        let variant = Variant::from(variant);
        self.shop.update_variant(variant);
        self.placement.update_variant(variant);
        self.fight.update_variant(variant);
    }

    /// Get hand for active player.
    pub fn get_hand(&self, index: usize) -> String {
        let c = self.placement.clone();
        let color = Color::from(index);
        self.shop.to_sfen(color)
    }

    /// Get confirmed players.
    pub fn get_confirmed(&self) -> [bool; 2] {
        self.confirmed()
    }

    fn confirmed(&self) -> [bool; 2] {
        let mut confirmed = [false, false];
        confirmed[0] = self.shop.is_confirmed(shuuro::Color::White);
        confirmed[1] = self.shop.is_confirmed(shuuro::Color::Black);
        confirmed
    }

    /// Buy one piece.
    pub fn buy_piece(
        &mut self,
        json: &GameGet,
        p: usize,
    ) -> Option<LiveGameMove> {
        if let Some(m) = Move::from_sfen(&json.game_move) {
            if let Move::Buy { piece } = m {
                return self.new_piece(piece, p, m);
            }
        } else {
            // If move is wrong then confirm player choice.
            self.shop.confirm(Color::from(p));
            return Some(LiveGameMove::BuyMove(self.confirmed()));
        }
        None
    }

    fn new_piece(
        &mut self,
        piece: Piece,
        player: usize,
        m: Move<S>,
    ) -> Option<LiveGameMove> {
        let player_color = Color::from(player);
        if player_color == piece.color {
            if let Some(confirmed) = self.shop.play(m) {
                self.game.draws = [false, false];
                self.game.hands[player] = self.shop.to_sfen(player_color);
                if confirmed[player_color as usize] {
                    return Some(LiveGameMove::BuyMove(confirmed));
                }
            }
        }
        None
    }

    /// DEPLOY PART

    pub fn set_deploy(&mut self, id: &String) -> Value {
        self.game.current_stage = 1;
        let hand = self.load_shop_hand();
        self.placement.generate_plinths();
        self.game.sfen = self.placement.to_sfen();
        self.game.side_to_move = 0;
        let value = serde_json::json!({
            "t": "redirect_deploy",
            "path": format!("/shuuro/1/{id}"),
            "hand": hand,
            "last_clock": Utc::now(),
            "side_to_move": "w",
            "w": String::from(&self.game.players[0]),
            "b": String::from(&self.game.players[1]),
            "sfen": self.game.sfen
        });
        value
    }

    /// Transfer hand from shop to deploy part.
    pub fn load_shop_hand(&mut self) -> String {
        let w = self.shop.to_sfen(Color::White);
        let b = self.shop.to_sfen(Color::Black);
        let hand = format!("{}{}", &w, &b);
        self.game.hands = [w, b];
        let sfen = P::empty_placement_board();
        self.placement.set_hand(hand.as_str());
        self.placement.set_sfen(&sfen).ok();
        hand
    }

    /// Get hands for both players.
    fn get_hands(&self) -> [String; 2] {
        [
            self.placement.get_hand(Color::White),
            self.placement.get_hand(Color::Black),
        ]
    }

    /// Check if deployment is over by checking player's hands.
    pub fn is_deployment_over(&mut self) -> bool {
        let mut completed: [bool; 3] = [false, false, false];
        let color_iter = Color::iter();
        for i in color_iter {
            completed[i.index()] =
                self.placement.is_hand_empty(i, PieceType::Plinth);
        }
        completed[2] = true;
        !completed.contains(&false)
    }

    pub fn place_move(
        &mut self,
        json: &GameGet,
        player: &String,
    ) -> Option<LiveGameMove> {
        if self.game.current_stage != 1 {
            return None;
        }
        if let Some(index) = self.player_index(&self.game.players, player) {
            if self.placement.side_to_move() != Color::from(index) {
                return None;
            }
            if let Some(clocks) = self.game.tc.click(index) {
                self.game.clocks = self.game.tc.clocks;
                self.game.last_clock = DT::now();
                return self.place_piece(json, index, clocks);
            } else {
            }
        }
        None
    }

    /// Placing piece on board. Returns LiveGameMove.
    fn place_piece(
        &mut self,
        json: &GameGet,
        index: usize,
        clocks: [u64; 2],
    ) -> Option<LiveGameMove> {
        match Move::from_sfen(json.game_move.as_str()) {
            Some(gm) => {
                if let Move::Put { to, piece } = gm {
                    if Color::from(index) == piece.color {
                        if let Some(s) = self.placement.place(piece, to) {
                            self.game.draws = [false, false];
                            let mut fme = false;
                            let m = s.split("_").next().unwrap().to_string();
                            let tf = self.is_deployment_over();
                            if tf {
                                fme = self.set_fight();
                            }
                            self.game.side_to_move = self
                                .other_index(self.placement.side_to_move())
                                as u8;
                            self.game.sfen = self.placement.generate_sfen();
                            self.game.hands = self.get_hands();
                            self.game.history.1.push(s);
                            return Some(LiveGameMove::PlaceMove(
                                m,
                                clocks,
                                fme,
                                tf,
                                self.game.players.clone(),
                            ));
                        }
                    } else {
                    }
                }
            }
            _ => (),
        }
        None
    }

    /// FIGHT PART

    /// Transfer data from deployment to fighting part of game.
    pub fn set_fight(&mut self) -> bool {
        self.game.current_stage = 2;
        self.game.tc.update_stage(2);
        self.game.last_clock = DT::now();
        let sfen = self.placement.generate_sfen();
        let outcome = self.fight.set_sfen(&sfen.as_str());
        if let Ok(_o) = outcome {
            self.update_status();
        }
        self.fight.in_check(self.fight.side_to_move().flip())
    }

    pub fn fight_move(
        &mut self,
        json: &GameGet,
        player: &String,
    ) -> Option<LiveGameMove> {
        if self.game.current_stage != 2 {
            return None;
        }
        if let Some(index) = self.player_index(&self.game.players, player) {
            if self.fight.side_to_move() != Color::from(index) {
                return None;
            }
            if let Some(clocks) = self.game.tc.click(index) {
                self.game.draws = [false, false];
                self.game.clocks = self.game.tc.clocks;
                self.game.last_clock = DT::now();
                return self.make_move(json, index, clocks);
            }
        }
        None
    }

    pub fn make_move(
        &mut self,
        json: &GameGet,
        index: usize,
        clocks: [u64; 2],
    ) -> Option<LiveGameMove> {
        #[allow(clippy::collapsible_match)]
        if let Some(gm) = Move::from_sfen(json.game_move.as_str()) {
            if let Move::Normal {
                from,
                to,
                promote: _,
            } = gm
            {
                if let Some(piece) = self.fight.piece_at(from) {
                    if Color::from(index) == piece.color
                        && self
                            .fight
                            .play(
                                from.to_string().as_str(),
                                to.to_string().as_str(),
                            )
                            .is_ok()
                    {
                        let outcome = self.update_status();
                        let stm = self.other_index(self.fight.side_to_move());
                        self.game.side_to_move = stm as u8;
                        self.game.sfen = self.fight.generate_sfen();
                        let m = self.fight.get_sfen_history().last().unwrap();
                        self.game.history.2.push(m.clone());
                        return Some(LiveGameMove::FightMove(
                            String::from(&json.game_move),
                            clocks,
                            self.game.status,
                            String::from(&self.game.result),
                            self.game.players.clone(),
                            outcome,
                        ));
                    }
                } else {
                }
            }
        }
        None
    }

    pub fn player_index(&self, p: &[String; 2], u: &String) -> Option<usize> {
        p.iter().position(|x| x == u)
    }

    fn other_index(&self, color: Color) -> usize {
        let b: bool = color as usize != 0;
        usize::from(!b)
    }

    /// DRAW PART

    pub fn draw_req(&mut self, username: &String) -> Option<(i8, [String; 2])> {
        if let Some(index) = self.player_index(&self.game.players, username) {
            self.game.draws[index] = true;
            if !self.game.draws.contains(&false) {
                self.game.status = 5;
                return Some((5, self.game.players.clone()));
            } else {
                return Some((-2, self.game.players.clone()));
            }
        }
        None
    }

    /// CLOCK PART

    /// After every 500ms, this function returns who lost on time.
    pub fn clock_status(
        &mut self,
        time_check: MutexGuard<TimeCheck>,
    ) -> Option<(Value, Value, [String; 2])> {
        if time_check.both_lost {
            self.game.status = 5;
        } else {
            self.game.status = 8;
            self.game.result = {
                if time_check.lost == 0 {
                    String::from("w")
                } else if time_check.lost == 1 {
                    String::from("b")
                } else {
                    String::from("")
                }
            };
        }
        let res = serde_json::json!({
            "t": "live_game_lot",
            "game_id": &self.game._id,
            "status": self.game.status,
            "result": String::from(&self.game.result)});
        let tv_res = serde_json::json!({"t": "live_game_end", "game_id": String::from(&self.game._id)});
        let tv_res = serde_json::json!({"t": "tv_game_update", "g": tv_res});
        drop(time_check);
        Some((res, tv_res, self.game.players.clone()))
    }

    /// Check clocks for current stage.
    pub fn check_clocks(&self, mut time_check: MutexGuard<TimeCheck>) {
        if self.game.current_stage == 0 {
            let durations = [
                self.game.tc.current_duration(0),
                self.game.tc.current_duration(1),
            ];
            let confirmed = self.confirmed();
            if durations == [None, None] {
                if confirmed == [false, false] {
                    time_check.both_lost();
                } else if let Some(confirmed) =
                    confirmed.iter().position(|i| i == &false)
                {
                    time_check.lost(confirmed);
                }
            } else if let Some(index) =
                durations.iter().position(|p| p.is_none())
            {
                time_check.lost(index);
            }
        } else if self.game.current_stage != 0 {
            let stm = self.game.side_to_move;
            if self.game.tc.current_duration(stm as usize).is_none() {
                time_check.lost(stm as usize);
            }
        }
        time_check.dont_exist();
    }

    /// After match is finished, update status.
    pub fn update_status(&mut self) -> String {
        let outcome = self.fight.outcome();
        match outcome {
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
        outcome.to_string()
    }

    pub fn get_game(&self) -> ShuuroGame {
        self.game.clone()
    }

    pub fn live_sfen(&self) -> (u8, String) {
        (self.game.current_stage, String::from(&self.game.sfen))
    }

    /// Resign if this player exist in game.
    pub fn resign(&mut self, username: &String) -> Option<[String; 2]> {
        if let Some(index) = self.player_index(&self.game.players, username) {
            self.game.status = 7;
            self.game.result = Color::from(index).to_string();
            self.game.tc.click(index);
            self.game.last_clock = DT::now();
            return Some(self.game.players.clone());
        }
        None
    }
}

pub type AllGames<S, B, A, P> =
    Arc<Mutex<HashMap<String, LiveGame<S, B, A, P>>>>;

#[derive(Clone, Debug)]
pub struct LiveGames<S, B, A, P>
where
    S: Square + Hash,
    B: BitBoard<S>,
    A: Attacks<S, B>,
    P: Position<S, B, A>,
    Self: Sized,
    for<'a> &'a B: BitOr<&'a B, Output = B>,
    for<'a> &'a B: BitAnd<&'a B, Output = B>,
    for<'a> &'a B: Not<Output = B>,
    for<'a> &'a B: BitOr<&'a S, Output = B>,
    for<'a> &'a B: BitAnd<&'a S, Output = B>,
{
    pub all: AllGames<S, B, A, P>,
    unfinished: Arc<Mutex<Vec<String>>>,
}

impl<S, B, A, P> LiveGames<S, B, A, P>
where
    S: Square + Hash,
    B: BitBoard<S>,
    A: Attacks<S, B>,
    P: Position<S, B, A>,
    Self: Sized,
    for<'a> &'a B: BitOr<&'a B, Output = B>,
    for<'a> &'a B: BitAnd<&'a B, Output = B>,
    for<'a> &'a B: Not<Output = B>,
    for<'a> &'a B: BitOr<&'a S, Output = B>,
    for<'a> &'a B: BitAnd<&'a S, Output = B>,
{
    /// Add new game to live games.
    pub fn add_game(&self, game: ShuuroGame) -> usize {
        let mut all = self.all.lock().unwrap();
        all.insert(String::from(&game._id), LiveGame::new(game));
        all.len()
    }

    /// Remove game after end.
    pub async fn remove_game(&self, db: &Collection<ShuuroGame>, id: &String) {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.remove(id) {
            let db = db.clone();
            let game = game.get_game();
            tokio::spawn(async move {
                update_entire_game(&db, &game).await;
            });
        }
    }

    /// Count all games.
    pub fn game_count(&self) -> usize {
        self.all.lock().unwrap().len()
    }

    /// Load games from db
    pub fn load_unfinished(&self, hm: &HashMap<String, ShuuroGame>) {
        let mut temp = HashMap::new();
        let mut v = vec![];
        for i in hm {
            //self.ws.players.new_spectators(&i.0);
            let mut game: LiveGame<S, B, A, P> = LiveGame::new(i.1.clone());
            let id = String::from(i.0);
            v.push(id.clone());
            if i.1.current_stage == 0 {
                let hands = format!("{}{}", &i.1.hands[0], &i.1.hands[1]);
                game.shop.set_hand(hands.as_str());
                temp.insert(id, game);
            } else if i.1.current_stage == 1 {
                game.placement.set_sfen_history(i.1.history.1.clone());
                let _ = game.placement.set_sfen(&i.1.sfen);
                temp.insert(id, game);
            } else if i.1.current_stage == 2 {
                game.fight.set_sfen_history(i.1.history.2.clone());
                let _ = game.fight.set_sfen(&i.1.sfen);
                temp.insert(id, game);
            }
        }
        *self.all.lock().unwrap() = temp;
        *self.unfinished.lock().unwrap() = v;
    }

    pub fn get_unfinished(&self) -> Vec<String> {
        let unfinished = self.unfinished.lock().unwrap();
        let games = unfinished.clone();
        drop(unfinished);
        games
    }

    pub fn del_unfinished(&self) {
        let mut unfinished = self.unfinished.lock().unwrap();
        *unfinished = vec![];
        drop(unfinished);
    }

    pub fn change_variant(&self, id: &String, variant: &String) {
        let mut all = self.all.lock().unwrap();
        if let Some(g) = all.get_mut(id) {
            g.change_variant(variant);
            drop(all);
        }
    }

    pub fn get_hand(&self, id: &String, user: &UserSession) -> Option<String> {
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(id) {
            if let Some(index) = g.player_index(&g.game.players, &user.username)
            {
                return Some(g.get_hand(index));
            }
        }
        None
    }

    pub fn get_confirmed(&self, id: &String) -> Option<[bool; 2]> {
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(id) {
            return Some(g.get_confirmed());
        }
        None
    }

    pub fn buy(&self, json: &GameGet, player: &String) -> Option<LiveGameMove> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            if let Some(p) = game.player_index(&game.game.players, player) {
                if let Some(_c) = game.game.tc.click(p) {
                    return game.buy_piece(json, p);
                } else {
                    return Some(LiveGameMove::LostOnTime(p));
                }
            }
        }
        None
    }

    pub fn place_move(
        &self,
        json: &GameGet,
        player: &String,
    ) -> Option<LiveGameMove> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            return game.place_move(json, player);
        }
        None
    }

    pub fn fight_move(
        &self,
        json: &GameGet,
        player: &String,
    ) -> Option<LiveGameMove> {
        let mut all = self.all.lock().unwrap();
        if let Some(game) = all.get_mut(&json.game_id) {
            return game.fight_move(json, player);
        }
        None
    }

    pub fn set_deploy(&self, id: &String) -> Option<Value> {
        if let Some(game) = self.all.lock().unwrap().get_mut(id) {
            return Some(game.set_deploy(id));
        }
        None
    }

    pub fn draw_req(
        &self,
        id: &String,
        username: &String,
    ) -> Option<(i8, [String; 2])> {
        if let Some(game) = self.all.lock().unwrap().get_mut(id) {
            return game.draw_req(username);
        }
        None
    }

    pub fn get_players(&self, id: &String) -> Option<[String; 2]> {
        if let Some(game) = self.all.lock().unwrap().get(id) {
            return Some(game.game.players.clone());
        }
        None
    }

    pub fn clock_status(
        &self,
        time_check: &Arc<Mutex<TimeCheck>>,
    ) -> Option<(Value, Value, [String; 2])> {
        let time_check = time_check.lock().unwrap();
        if let Some(g) = self
            .all
            .lock()
            .unwrap()
            .get_mut(&String::from(&time_check.id))
        {
            return g.clock_status(time_check);
        }
        drop(time_check);
        None
    }

    pub fn check_clocks(&self, time_check: &Arc<Mutex<TimeCheck>>) {
        let time_check = time_check.lock().unwrap();
        let id = String::from(&time_check.id);
        if let Some(game) = self.all.lock().unwrap().get(&id) {
            game.check_clocks(time_check);
        }
    }

    pub async fn get_game<'a>(
        &self,
        id: &String,
        _db: &Collection<ShuuroGame>,
        s: &'a MessageHandler<'a>,
    ) -> Option<ShuuroGame> {
        let id = String::from(id);
        let all = self.all.lock().unwrap();
        if let Some(g) = all.get(&id) {
            return Some(g.game.clone());
        }
        let _ = s.db_tx.clone().send(MsgDatabase::GetGame(id));

        None
    }

    pub fn live_sfen(&self, id: &String) -> Option<(u8, String)> {
        if let Some(g) = self.all.lock().unwrap().get(id) {
            return Some(g.live_sfen());
        }
        None
    }

    pub fn resign(
        &self,
        id: &String,
        username: &String,
    ) -> Option<[String; 2]> {
        if let Some(g) = self.all.lock().unwrap().get_mut(id) {
            return g.resign(username);
        }
        None
    }

    /// Get 20 matches for tv.
    pub fn get_tv(&self) -> Vec<TvGame> {
        let c = 0;
        let mut games = vec![];
        let all = self.all.lock().unwrap();
        for i in all.iter() {
            if c == 20 {
                break;
            }
            let f = &i.1.game.sfen;
            if f == "" {
                continue;
            }
            let id = &i.1.game._id;
            let w = &i.1.game.players[0];
            let b = &i.1.game.players[1];
            let t = "live_tv";
            let tv = TvGame::new(t, id, w, b, f);
            games.push(tv);
        }
        games
    }

    /// Before closing server save on exit.
    pub async fn save_on_exit(&self, db: &Collection<ShuuroGame>) {
        let all = self.all.lock().unwrap().clone();
        for (_, game) in all {
            update_entire_game(db, &game.game).await;
        }
    }
}

impl<S, B, A, P> Default for LiveGames<S, B, A, P>
where
    S: Square + Hash,
    B: BitBoard<S>,
    A: Attacks<S, B>,
    P: Position<S, B, A>,
    Self: Sized,
    for<'a> &'a B: BitOr<&'a B, Output = B>,
    for<'a> &'a B: BitAnd<&'a B, Output = B>,
    for<'a> &'a B: Not<Output = B>,
    for<'a> &'a B: BitOr<&'a S, Output = B>,
    for<'a> &'a B: BitAnd<&'a S, Output = B>,
{
    /// Load games from db
    fn default() -> Self {
        A::init();
        Self {
            all: arc2(HashMap::new()),
            unfinished: arc2(vec![]),
        }
    }
}
