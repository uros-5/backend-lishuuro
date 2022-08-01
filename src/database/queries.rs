use mongodb::Collection;

use crate::lichess::login::random_username;

use super::{redis::UserSession, Player};

use bson::{doc, DateTime};

pub async fn create_player(db: &Collection<Player>) -> String {
    loop {
        let username = random_username();
        let player = Player {
            _id: String::from(&username),
            reg: false,
            created_at: bson::DateTime::now(),
        };
        let res = db.insert_one(&player, None).await;
        if let Ok(_) = res {
            return format!("{}", &username);
        }
    }
}

pub async fn player_exist(
    db: &Collection<Player>,
    username: &str,
    session: &UserSession,
) -> Option<UserSession> {
    let exist = db
        .find_one(doc! {"_id": String::from(username)}, None)
        .await;

    if let Ok(player) = exist {
        let mut session = UserSession::from(session);
        session.is_new = true;
        session.username(username);
        session.register();
        if let Some(_) = player{
            return Some(session);
        } else {
            let player = Player::from(&session);
            let player = db.insert_one(player, None).await;
            return Some(session);
        }
    }
    None
}
