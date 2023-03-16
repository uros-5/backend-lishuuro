use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::{rooms::ChatMsg, time_control::TimeCheck};

/// This struct is used for most game moves.
#[derive(Clone, Serialize, Deserialize)]
pub struct GameGet {
    pub t: String,
    pub game_id: String,
    #[serde(default)]
    pub game_move: String,
    pub variant: String,
}

pub enum LiveGameMove {
    BuyMove([bool; 2]),
    LostOnTime(usize),
    PlaceMove(String, [u64; 2], bool, bool, [String; 2], String),
    FightMove(String, [u64; 2], i32, String, [String; 2], String, String),
}

#[derive(Clone)]
pub enum MsgDatabase {
    GetGame(String),
    LostOnTime(Arc<Mutex<TimeCheck>>),
    InsertGameMove(GameGet),
}

impl From<&ChatMsg> for GameGet {
    fn from(value: &ChatMsg) -> Self {
        GameGet {
            t: String::from(""),
            game_id: String::from(&value.id),
            game_move: String::from(""),
            variant: String::from(&value.variant),
        }
    }
}

impl GameGet {
    pub fn new(id: &String, variant: &String) -> Self {
        Self {
            t: String::from(""),
            game_id: String::from(id),
            game_move: String::from(""),
            variant: String::from(variant),
        }
    }
}
