use mongodb::Collection;
use serde_json::Value;

use crate::lichess::login::{random_game_id, random_username};

use super::{
    mongo::{Player, ShuuroGame},
    redis::UserSession,
};

use bson::doc;

/// Create new player.
pub async fn create_player(db: &Collection<Player>) -> String {
    loop {
        let username = random_username();
        let player = Player {
            _id: String::from(&username),
            reg: false,
            created_at: bson::DateTime::now(),
        };
        let res = db.insert_one(&player, None).await;
        // Player is added, therefore it's new.
        if let Ok(_) = res {
            return format!("{}", &username);
        }
    }
}

/// Check if player(with lichess account) exist
pub async fn player_exist(
    db: &Collection<Player>,
    username: &str,
    session: &UserSession,
) -> Option<UserSession> {
    let exist = db
        .find_one(doc! {"_id": String::from(username)}, None)
        .await;

    if let Ok(player) = exist {
        let mut session = session.clone();
        session.is_new = true;
        session.new_username(username);
        session.new_register();
        if let Some(_) = player {
            // Player exist.
            return Some(session);
        } else {
            let player = Player::from(&session);
            let _player = db.insert_one(player, None).await;
            return Some(session);
        }
    }
    None
}

pub async fn game_exist(db: &Collection<ShuuroGame>) -> String {
    loop {
        let id = random_game_id();
        if let Some(g) = get_game_db(db, &id).await {
            continue;
        }
        return id;
    }
}

pub async fn get_game_db(db: &Collection<ShuuroGame>, id: &String) -> Option<ShuuroGame> {
    let filter = doc! {"_id": id};
    if let Ok(r) = db.find_one(filter, None).await {
        if let Some(g) = r {
            return Some(g);
        }
    }
    None
}

pub async fn add_game_to_db(db: &Collection<ShuuroGame>, game: &ShuuroGame) -> Value {
    if let Err(res) = db.insert_one(game, None).await {}
    serde_json::json!({"t": "live_game_start", "game_id": &game._id, "game_info": &game})
}
