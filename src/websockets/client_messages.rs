use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct GameGet {
    pub t: String,
    pub game_id: String,
}