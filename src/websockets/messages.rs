use actix::prelude::{Message, Recipient};
use mongodb::Collection;

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
#[rtype(result = "bool")]
pub struct GameMessage {
    pub message_type: GameMessageType,
}

impl GameMessage {
    pub fn new_adding_game(game_id: String, users: [String; 2], shuuro_game: ShuuroGame) -> Self {
        let message_type = GameMessageType::new_adding_game(game_id, users, shuuro_game);
        Self { message_type }
    }

    pub fn time_check(game_id: &String) -> Self {
        let message_type = GameMessageType::time_check(game_id);
        Self { message_type }
    }

    pub fn lost_on_time(game_id: &String) -> Self {
        let message_type = GameMessageType::lost_on_time(game_id);
        Self { message_type }
    }

    pub fn remove_game(game_id: &String) -> Self {
        let message_type = GameMessageType::remove_game(game_id);
        Self { message_type }
    }

    pub fn start_all(games: Vec<(String, ShuuroGame)>) -> Self {
        let message_type = GameMessageType::start_all(games);
        Self { message_type }
    }

    pub fn save_all() -> Self {
        let message_type = GameMessageType::save_all();
        Self { message_type }
    }
}

pub enum GameMessageType {
    AddingGame {
        game_id: String,
        users: [String; 2],
        shuuro_game: ShuuroGame,
    },
    StartAll {
        games: Vec<(String, ShuuroGame)>,
    },
    TimeCheck {
        game_id: String,
    },
    LostOnTime {
        game_id: String,
    },
    RemoveGame {
        game_id: String,
    },
    SaveAll,
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

    pub fn time_check(game_id: &String) -> Self {
        Self::TimeCheck {
            game_id: String::from(game_id),
        }
    }

    pub fn lost_on_time(game_id: &String) -> Self {
        Self::LostOnTime {
            game_id: String::from(game_id),
        }
    }

    pub fn remove_game(game_id: &String) -> Self {
        Self::RemoveGame {
            game_id: String::from(game_id),
        }
    }

    pub fn start_all(games: Vec<(String, ShuuroGame)>) -> Self {
        Self::StartAll { games }
    }

    pub fn save_all() -> Self {
        Self::SaveAll
    }
}

#[derive(Message)]
#[rtype(result = "{}")]
pub struct News {
    pub news: Vec<NewsItem>,
    pub active_player: ActivePlayer,
}
impl News {
    pub fn news(active_player: ActivePlayer, news: Vec<NewsItem>) -> Self {
        Self {
            news,
            active_player,
        }
    }
}

#[derive(Message)]
#[rtype(result = "(Vec<(String, ShuuroGame)>, Collection<ShuuroGame>)")]
pub struct Games {}
