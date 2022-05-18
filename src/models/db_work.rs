use crate::websockets::lobby::Lobby;
use crate::websockets::messages::{GameMessage, Games, News, Restart};
use actix::prelude::Context;
use actix::{Addr, AsyncContext};
use bson::{doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use futures::Future;
use mongodb::Collection;
use mongodb::options::FindOptions;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use super::model::{ActivePlayer, NewsItem, ShuuroGame, User};

/// GAMES PART

/// game from live is used here(after match is done)
pub fn update_entire_game(
    db: Collection<ShuuroGame>,
    id: &String,
    game: &ShuuroGame,
) -> impl Future<Output = ()> {
    let all = vec![(String::from(id), game.clone())]; 
        let f = Box::pin(async move {
            update_all(all, db).await;
    });
    f
}


/// get all unfinished matches
pub async fn unfinished(db: &Collection<ShuuroGame>) -> Vec<(String, ShuuroGame)> {
    let db = db.clone();
    let filter = doc! {"status" : {"$lt": &0}};
    let mut hm: Vec<(String, ShuuroGame)> = vec![];
    let c = db.find(filter, None);
    if let Ok(c) = c.await {
        let games: Vec<ShuuroGame> = c.try_collect().await.unwrap_or_else(|_| vec![]);
        for g in games {
            hm.push((String::from(&g.game_id), g));
        }
    }
    return hm;
}

/// get one game
pub fn get_game<'a>(
    lobby: Lobby,
    game_id: String,
    player: ActivePlayer,
) -> impl Future<Output = ()> + 'a {
    let filter = doc! {"_id": ObjectId::from_str(game_id.as_str()).unwrap()};
    let b = Box::pin(async move {
        let db =&lobby.db_shuuro_games;
        let game = db.find_one(filter, None);
        if let Ok(g) = game.await {
            if let Some(g) = g {
                let res = serde_json::json!({"t": "live_game_start", "game_id": game_id, "game_info": &g.clone()});
                lobby.send_message(&player, res);
            }
        }
    });
    b
}



/// save current state before restart
pub async fn save_state(address: Addr<Lobby>) {
    address.do_send(Restart);
    let games = address.send(Games {}).await;
    if let Ok(res) = games {
        update_all(res.0, res.1).await;
    }
}

pub async fn update_all(all: Vec<(String, ShuuroGame)>, db: Collection<ShuuroGame>) {
    for i in all {
        let filter = doc! {"_id": ObjectId::from_str(i.0.as_str()).unwrap()};
        let update = doc! {"$set": bson::to_bson(&i.1).unwrap()};
        let game1 = db.find_one_and_update(filter, update, None);
        match game1.await {
            _g => {}
        };
    }
}




pub fn new_game<'a>(
    ctx: &Context<Lobby>,
    shuuro_games: Collection<ShuuroGame>,
    users: [String; 2],
    shuuro_game: ShuuroGame,
    users_db: Collection<User>,
) -> impl Future<Output = ()> + 'a {
    fn oid(id: String) -> String {
        id.replace("ObjectId(\"", "").replace("\")", "")
    }
    let ctx = ctx.address().clone();
    let mut shuuro_game = shuuro_game.clone();
    let b = Box::pin(async move {
        let game_added = shuuro_games.insert_one(&shuuro_game, None);
        match game_added.await {
            g => {
                let ratings = user_ratings(&users_db, &users).await;
                if let Some(ratings) = ratings {
                    shuuro_game.update_ratings(ratings);
                }
                let id = g.ok().unwrap().inserted_id.to_string();
                let game_id = oid(id);
                let filter = doc! {"_id": ObjectId::from_str(&game_id).unwrap()};
                let update = doc! {"$set": bson::to_bson(&shuuro_game).unwrap()};
                shuuro_games.update_one(filter, update, None).await;
                ctx.do_send(GameMessage::new_adding_game(
                    game_id.clone(),
                    users.clone(),
                    shuuro_game,
                ));
            }
        }
    });
    b
}

pub async fn user_ratings(
    col: &Collection<User>,
    users: &[String; 2],
) -> Option<HashMap<String, [f64; 2]>> {
    let mut ratings: HashMap<String, [f64; 2]> = HashMap::new();
    let players = col
        .find(doc! {"$or": [{"_id": &users[0]}, {"_id": &users[1]}]}, None)
        .await;
    if let Ok(cursor) = players {
        let players: Vec<User> = cursor.try_collect().await.unwrap_or_else(|_| vec![]);
        if players.len() != 2 {
            return None;
        }
        for user in players.iter() {
            ratings.insert(user._id.clone(), [user.rating, user.deviation]);
        }
        return Some(ratings);
    }
    None
}


pub fn start_clock(ctx: Addr<Lobby>, game_id: &String) {
    let game_id = String::from(game_id);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1500));
        loop {
            interval.tick().await;
            let time = ctx.send(GameMessage::time_check(&game_id)).await;
            if let Ok(time) = time {
                if let Some(time) = time {
                    if !time {
                        ctx.send(GameMessage::lost_on_time(&game_id)).await;
                        ctx.send(GameMessage::remove_game(&game_id)).await;
                        break;
                    }
                }
            } else {
                break;
            }
        }
    });
}


pub fn get_home_news(
    ctx: &Context<Lobby>,
    news: &Collection<NewsItem>,
    player: &ActivePlayer,
) -> impl Future<Output = ()> {
    let ctx2 = ctx.address().clone();
    let news = news.clone();
    let active_player = player.clone();
    let f = Box::pin(async move {
        let all = news.find(doc! {}, None).await;
        if let Ok(c) = all {
            let news_: Vec<NewsItem> = c.try_collect().await.unwrap_or_else(|_| vec![]);
            ctx2.do_send(News::news(active_player, news_));
            //ctx2.send_message(&msg.player, res)
        }
    });
    f
}