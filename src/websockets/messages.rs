use actix::prelude::{Message, Recipient};

use crate::models::model::{ActivePlayer, NewsItem, ShuuroGame};

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

#[derive(Message)]
#[rtype(result = "{}")]
pub struct RegularMessage {
    pub text: String,
    pub player: ActivePlayer,
}

impl RegularMessage {
    pub fn new(text: String, username: &String, logged: &bool) -> Self {
        RegularMessage {
            text,
            player: ActivePlayer::new(logged, username),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub player: ActivePlayer,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub player: ActivePlayer,
}

#[derive(Message)]
#[rtype(result = "{}")]
pub struct GameMessage {
    pub message_type: GameMessageType,
}

pub enum GameMessageType {
    AddingGame {
        game_id: String,
        users: [String; 2],
        shuuro_game: ShuuroGame,
    },
    News {
        news: Vec<NewsItem>,
        active_player: ActivePlayer,
    },
}

impl GameMessageType {
    pub fn new_adding_game(
        game_id: String,
        users: [String; 2],
        shuuro_game: ShuuroGame,
    ) -> GameMessageType {
        GameMessageType::AddingGame {
            game_id,
            users,
            shuuro_game,
        }
    }

    pub fn news(active_player: ActivePlayer, news: Vec<NewsItem>) -> GameMessageType {
        GameMessageType::News {
            news,
            active_player,
        }
    }
}
