use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::time_control::TimeCheck;

#[derive(Clone, Serialize, Deserialize)]
pub struct GameGet {
    pub t: String,
    pub game_id: String,
    #[serde(default)]
    pub game_move: String,
}

pub enum LiveGameMove {
    BuyMove([bool; 2]),
    LostOnTime(usize),
    PlaceMove(String, [u64; 2], bool, bool, [String; 2]),
    FightMove(String, [u64; 2], i32, String, [String; 2], String),
}

#[derive(Clone)]
pub enum MsgDatabase {
    GetGame(String),
    LostOnTime(Arc<Mutex<TimeCheck>>),
}
