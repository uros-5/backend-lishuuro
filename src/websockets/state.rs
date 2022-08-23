use super::{
    rooms::{ChatRooms, Players},
    ClientMessage, GameReqs,
};
use tokio::sync::broadcast;

pub struct WsState {
    pub players: Players,
    pub chat: ChatRooms,
    pub game_reqs: GameReqs,
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
        }
    }
}
