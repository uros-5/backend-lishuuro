use crate::database::{mongo::ShuuroGame, queries::unfinished};

use super::{
    rooms::{ChatRooms, Players},
    ClientMessage, GameReqs, ShuuroGames,
};
use mongodb::Collection;
use tokio::sync::broadcast::{self};

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
    pub async fn load_unfinished(&self, db: &Collection<ShuuroGame>) {
        let unfinished = unfinished(db).await;
        self.shuuro_games.load_unfinished(unfinished);
    }
}
