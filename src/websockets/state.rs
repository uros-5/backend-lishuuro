use std::collections::HashMap;

use crate::database::{mongo::ShuuroGame, queries::unfinished};

use super::{
    games::ShuuroGames,
    rooms::{ChatRooms, Players},
    ClientMessage, GameReqs,
};
use mongodb::Collection;
use tokio::sync::broadcast;

/// This struct contains all data.
pub struct WsState {
    pub players: Players,
    pub chat: ChatRooms,
    pub game_reqs: GameReqs,
    pub shuuro_games: ShuuroGames,
    pub tx: broadcast::Sender<ClientMessage>,
}

impl Default for WsState {
    fn default() -> Self {
        let players = Players::default();
        let chat = ChatRooms::default();
        let game_reqs = GameReqs::default();
        let (tx, _rx) = broadcast::channel(100);
        Self {
            players,
            chat,
            game_reqs,
            tx,
            shuuro_games: ShuuroGames::default(),
        }
    }
}

impl WsState {
    /// Load all games that are not finished.
    pub async fn load_unfinished(&self, db: &Collection<ShuuroGame>) {
        let unfinished = unfinished(db).await;
        let mut games8 = HashMap::new();
        let mut games12 = HashMap::new();
        self.players.add_spectators(&unfinished);
        for game in unfinished {
            if game.1.variant.contains("shuuro") {
                games12.insert(game.0, game.1);
            } else {
                games8.insert(game.0, game.1);
            }
        }
        let unfinished = vec![games8, games12];
        self.shuuro_games.load_unfinished(unfinished);
    }
}
