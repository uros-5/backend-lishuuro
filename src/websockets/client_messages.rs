// WEB SOCKETS MESSAGES

use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct GameRequest {
    pub t: String,
    pub color: String,
    pub game_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameMove {
    pub t: String,
    pub game_id: String,
    pub game_move: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameGetHand {
    pub t: String,
    pub game_id: String,
    pub color: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameGet {
    pub t: String,
    pub game_id: String,
}