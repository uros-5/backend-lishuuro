use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mongodb::Collection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shuuro::{
    shuuro12::{
        attacks12::Attacks12, bitboard12::BB12, position12::P12,
        square12::Square12,
    },
    shuuro8::{
        attacks8::Attacks8, bitboard8::BB8, position8::P8, square8::Square8,
    },
};

use crate::database::{mongo::ShuuroGame, redis::UserSession};

use super::{
    live_game::LiveGames, time_control::TimeCheck, GameGet, LiveGameMove,
    MessageHandler,
};

// macro_rules! tv {

// }

macro_rules! send {
    (0, $self: ident, $json: expr, $method: ident, $($params:expr),*) => {
        if $json.variant.contains("shuuro") {
            $self.live_games12.$method($($params),*)
        } else {
            $self.live_games8.$method($($params),*)
        }
    };

    (1, $self: ident, $json: expr, $method: ident, $($params:expr),*) => {
        if $json.variant.contains("shuuro") {
            $self.live_games12.$method($($params),*).await
        } else {
            $self.live_games8.$method($($params),*).await
        }
    };
}

type Live8 = LiveGames<
    Square8,
    BB8<Square8>,
    Attacks8<Square8, BB8<Square8>>,
    P8<Square8, BB8<Square8>>,
>;

type Live12 = LiveGames<
    Square12,
    BB12<Square12>,
    Attacks12<Square12, BB12<Square12>>,
    P12<Square12, BB12<Square12>>,
>;

#[derive(Default)]
pub struct ShuuroGames {
    pub live_games8: Live8,
    pub live_games12: Live12,
}

impl ShuuroGames {
    /// Add new game to live games.
    pub fn add_game(&self, game: ShuuroGame, with_value: bool) -> ShuuroGame {
        send!(0, self, game, add_game, game, with_value)
    }
    /// Remove game after end.
    pub async fn remove_game(
        &self,
        json: &GameGet,
        db: &Collection<ShuuroGame>,
    ) {
        send!(1, self, json, remove_game, db, &json.game_id);
    }

    /// Count all games.
    pub fn game_count(&self) -> usize {
        let first = self.live_games8.game_count();
        let second = self.live_games12.game_count();
        first + second
    }

    /// Load games from db
    /// First game is for `P8`
    pub fn load_unfinished(&self, games: Vec<HashMap<String, ShuuroGame>>) {
        // let variants = ["shuuro", "standard"]; //, "shuuroFairy", "standardFairy"];
        for (i, g) in games.iter().enumerate() {
            if i == 0 {
                self.live_games8.load_unfinished(g);
            } else {
                self.live_games12.load_unfinished(g);
            }
        }
    }

    pub fn get_unfinished(&self) -> [Vec<String>; 2] {
        [
            self.live_games8.get_unfinished(),
            self.live_games12.get_unfinished(),
        ]
    }

    pub fn delete_unfinished(&self) {
        self.live_games8.del_unfinished();
        self.live_games12.del_unfinished();
    }

    // SHOP PART

    pub fn change_variant(&self, json: &GameGet) {
        send!(0, self, json, change_variant, &json.game_id, &json.variant)
    }
    /// Get hand for active player.
    pub fn get_hand(
        &self,
        json: &GameGet,
        user: &UserSession,
    ) -> Option<String> {
        send!(0, self, json, get_hand, &json.game_id, user)
    }

    pub fn get_confirmed(&self, json: &GameGet) -> Option<[bool; 2]> {
        send!(0, self, json, get_confirmed, &json.game_id)
    }

    pub fn buy(&self, json: &GameGet, player: &String) -> Option<LiveGameMove> {
        send!(0, self, json, buy, json, player)
    }

    // DEPLOY PART

    pub fn place_move(
        &self,
        json: &GameGet,
        player: &String,
    ) -> Option<LiveGameMove> {
        send!(0, self, json, place_move, json, player)
    }

    pub fn fight_move(
        &self,
        json: &GameGet,
        player: &String,
    ) -> Option<LiveGameMove> {
        send!(0, self, json, fight_move, json, player)
    }

    pub fn set_deploy(&self, json: &GameGet) -> Option<Value> {
        send!(0, self, json, set_deploy, &json.game_id)
    }

    /// DRAW PART

    pub fn draw_req(
        &self,
        json: &GameGet,
        username: &String,
    ) -> Option<(i8, [String; 2])> {
        send!(0, self, json, draw_req, &json.game_id, username)
    }
    pub fn get_players(&self, json: &GameGet) -> Option<[String; 2]> {
        send!(0, self, json, get_players, &json.game_id)
    }

    /// CLOCK PART

    /// After every 500ms, this function returns who lost on time.
    pub fn clock_status(
        &self,
        json: &GameGet,
        time_check: &Arc<Mutex<TimeCheck>>,
    ) -> Option<(Value, Value, [String; 2])> {
        send!(0, self, json, clock_status, time_check)
    }

    /// Check clocks for current stage.
    pub fn check_clocks(
        &self,
        json: &GameGet,
        time_check: &Arc<Mutex<TimeCheck>>,
    ) {
        send!(0, self, json, check_clocks, time_check);
    }

    pub async fn get_game<'a>(
        &self,
        json: &GameGet,
        db: &Collection<ShuuroGame>,
        s: &'a MessageHandler<'a>,
    ) -> Option<ShuuroGame> {
        let mut json = json.clone();
        // json.variant
        if let Some(game) =
            send!(1, self, json, get_game, &json.game_id, db, s, false)
        {
            return Some(game);
        }
        json.variant = String::from("standard");
        if let Some(game) =
            send!(1, self, json, get_game, &json.game_id, db, s, false)
        {
            return Some(game);
        }
        send!(1, self, json, get_game, &json.game_id, db, s, true)
    }

    pub fn live_sfen(&self, json: &GameGet) -> Option<(u8, String)> {
        send!(0, self, json, live_sfen, &json.game_id)
    }

    /// Resign if this player exist in game.
    pub fn resign(
        &self,
        json: &GameGet,
        username: &String,
    ) -> Option<[String; 2]> {
        send!(0, self, json, resign, &json.game_id, username)
    }

    pub async fn save_on_exit(&self, games: &Collection<ShuuroGame>) {
        self.live_games8.save_on_exit(games).await;
        self.live_games12.save_on_exit(games).await;
    }

    pub fn get_tv(&self) -> Vec<TvGame> {
        let mut all8 = self.live_games8.get_tv();
        let mut all12 = self.live_games12.get_tv();
        all8.append(&mut all12);
        all8
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TvGame {
    pub game_id: String,
    pub w: String,
    pub b: String,
    pub sfen: String,
    pub variant: String,
}

impl TvGame {
    pub fn new(
        game_id: &str,
        w: &str,
        b: &str,
        fen: &str,
        variant: String,
    ) -> Self {
        Self {
            game_id: String::from(game_id),
            w: String::from(w),
            b: String::from(b),
            sfen: String::from(fen),
            variant,
        }
    }
}
