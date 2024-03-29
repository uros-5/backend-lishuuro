use std::collections::HashMap;

use futures::TryStreamExt;
use mongodb::{options::FindOptions, Collection};
use serde_json::Value;

use crate::{
    lichess::login::{random_game_id, random_username},
    websockets::{server_messages::live_game_start, GameGet},
};

use super::{
    mongo::{Article, Player, ProfileGame, ShuuroGame},
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
        if res.is_ok() {
            return (username).to_string();
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
        if player.is_some() {
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

/// Check if game ID exist.
pub async fn game_exist(db: &Collection<ShuuroGame>) -> String {
    loop {
        let id = random_game_id();
        if (get_game_db(db, &id).await).is_some() {
            continue;
        }
        return id;
    }
}

/// Get game from database if it exist.
pub async fn get_game_db(
    db: &Collection<ShuuroGame>,
    id: &String,
) -> Option<ShuuroGame> {
    let id = String::from(id);
    let filter = doc! {"_id": id};
    match db.find_one(filter, None).await {
        Ok(r) => {
            if let Some(g) = r {
                return Some(g);
            }
        }
        _ => (),
    }
    None
}

/// Add new game to database and format ws message.
pub async fn add_game_to_db(
    db: &Collection<ShuuroGame>,
    game: &ShuuroGame,
) -> Value {
    if let Err(_res) = db.insert_one(game, None).await {}
    live_game_start(game)
}

/// Update all fields for game.
pub async fn update_entire_game(
    db: &Collection<ShuuroGame>,
    game: &ShuuroGame,
) {
    let query = doc! {"_id": &game._id};
    let update = doc! {"$set": bson::to_bson(&game).unwrap()};
    db.update_one(query, update, None).await.ok();
}

/// Get last 5 games for player.
pub async fn get_player_games(
    db: &Collection<ShuuroGame>,
    username: &String,
    page: u64,
) -> Option<Vec<ProfileGame>> {
    let options = FindOptions::builder()
        .projection(doc! {"history": 0, "credits": 0, "hands": 0})
        .sort(doc! {"last_clock": -1})
        .skip(Some(page * 5))
        .limit(Some(5))
        .build();
    let filter = doc! {"players": {"$in": [username]}};
    let q = db
        .clone_with_type::<ProfileGame>()
        .find(filter, options)
        .await;
    if let Ok(res) = q {
        let games: Vec<ProfileGame> =
            res.try_collect().await.unwrap_or_else(|_| vec![]);
        return Some(games);
    }
    None
}

/// Get article if ID exist.
pub async fn get_article(
    db: &Collection<Article>,
    id: &String,
) -> Option<Article> {
    let filter = doc! {"_id": id};
    if let Ok(n) = db.find_one(filter, None).await {
        if let Some(n) = n {
            return Some(n);
        }
    }
    None
}

/// get all unfinished matches
pub async fn unfinished(
    db: &Collection<ShuuroGame>,
) -> HashMap<String, ShuuroGame> {
    let filter = doc! {"status" : {"$lt": &0}};
    let mut hm = HashMap::new();
    let c = db.find(filter, None);
    if let Ok(c) = c.await {
        let games: Vec<ShuuroGame> =
            c.try_collect().await.unwrap_or_else(|_| vec![]);
        for g in games {
            hm.insert(String::from(&g._id), g);
        }
    }
    hm
}

/// push new player move to history array
pub async fn insert_move(db: &Collection<ShuuroGame>, json: &GameGet) {
    let query = doc! {"_id": &json.game_id};
    let field = {
        if json.game_move.contains('@') {
            1
        } else {
            2
        }
    };
    let field = format!("history.{}", field);
    let update = doc! {"$push": {field: &json.game_move}};
    db.update_one(query, update, None).await.ok();
}
