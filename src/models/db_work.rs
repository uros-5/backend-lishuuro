use crate::websockets::lobby::Lobby;
use crate::websockets::messages::{GameMessage, Games, News};
use actix::prelude::Context;
use actix::{Addr, AsyncContext};
use bson::{doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use futures::Future;
use glicko2::{new_rating, GameResult, GlickoRating};
use mongodb::Collection;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use super::model::{ActivePlayer, NewsItem, PlayerMatch, ShuuroGame, User};

macro_rules! gr {
    ($l:expr, $res:expr) => {
        PlayerMatch::new($l, $res)
    };
}

macro_rules! gr2 {
    ($result:expr, $wr:expr, $br:expr) => {{
        let wr_n: PlayerMatch;
        let br_n: PlayerMatch;
        if $result == "w" {
            wr_n = gr!($wr, "w");
            br_n = gr!($br, "l");
        } else {
            wr_n = gr!($wr, "l");
            br_n = gr!($br, "w");
        }
        (wr_n, br_n)
    }};
}

pub fn update_entire_game(
    s: &Lobby,
    id: &String,
    game: &ShuuroGame,
    end: bool,
) -> impl Future<Output = ()> {
    let filter = doc! {"_id": ObjectId::from_str(id.as_str()).unwrap()};
    let update = doc! {"$set": bson::to_bson(&game).unwrap()};
    let game = game.clone();
    let self2 = s.clone();
    let b = Box::pin(async move {
        let game1 = self2
            .db_shuuro_games
            .find_one_and_update(filter, update, None);
        match game1.await {
            _g => {
                if end {
                    let wr = game.ratings.get(&game.white).unwrap();
                    let br = game.ratings.get(&game.white).unwrap();

                    let mut wrn = gr!(wr, "d");
                    let mut brn = gr!(br, "d");
                    if [3, 4, 5, 6, 8].contains(&game.status) {
                        wrn = PlayerMatch::new(wr, "d");
                    } else if game.status == 1 {
                        (wrn, brn) = gr2!(&game.result, wr, br);
                    } else if game.status == 7 {
                        (wrn, brn) = gr2!(&game.result, wr, br);
                    }
                    add_result(&self2.db_users, &wrn, &game.white).await;
                    add_result(&self2.db_users, &brn, &game.black).await;
                }
            }
        };
    });
    b
}

pub fn get_all(db: &Collection<ShuuroGame>) -> impl Future<Output = Vec<(String, ShuuroGame)>> {
    let db = db.clone();
    let filter = doc! {"status" : {"$lt": 0}};
    let b = Box::pin(async move {
        let mut hm: Vec<(String, ShuuroGame)> = vec![];
        let c = db.find(filter, None);
        if let Ok(c) = c.await {
            let games: Vec<ShuuroGame> = c.try_collect().await.unwrap_or_else(|_| vec![]);
            for g in games {
                hm.push((g.game_id.clone(), g));
            }
        }
        return hm;
    });
    b
}

pub async fn add_result(users: &Collection<User>, m: &PlayerMatch, username: &String) {
    let filter = doc! { "_id": username };
    let update = doc! {"$push": {"last_games": bson::to_bson(m).unwrap()}};
    if let Ok(c) = users.find_one_and_update(filter, update, None).await {
        if let Some(user) = c {
            let mut temp: &Vec<PlayerMatch> = &vec![];
            if user.last_games.len() <= 15 {
                temp = &user.last_games;
            }
            let nr = new_ratings(m.r, m.d, temp);
            let update = doc! {"$set": {"rating": nr.value, "deviation": nr.deviation}};
            let filter = doc! { "_id": username };
            users.find_one_and_update(filter, update, None).await;
        }
    }
}

pub fn get_home_news(
    ctx: &Context<Lobby>,
    news: &Collection<NewsItem>,
    player: &ActivePlayer,
) -> impl Future<Output = ()> {
    let ctx2 = ctx.address().clone();
    let news = news.clone();
    let active_player = player.clone();
    let b = Box::pin(async move {
        let all = news.find(doc! {}, None).await;
        if let Ok(c) = all {
            let news_: Vec<NewsItem> = c.try_collect().await.unwrap_or_else(|_| vec![]);
            ctx2.do_send(News::news(active_player, news_));
            //ctx2.send_message(&msg.player, res)
        }
    });
    b
}

pub fn get_game<'a>(
    self1: &Lobby,
    game_id: String,
    player: ActivePlayer,
) -> impl Future<Output = ()> + 'a {
    let filter = doc! {"_id": ObjectId::from_str(game_id.as_str()).unwrap()};
    let db = self1.db_shuuro_games.clone();
    let self2 = self1.clone();
    let game_id = game_id.clone();

    let b = Box::pin(async move {
        let game = db.find_one(filter, None);
        if let Ok(g) = game.await {
            if let Some(g) = g {
                let res = serde_json::json!({"t": "live_game_start", "game_id": game_id, "game_info": &g.clone()});
                self2.send_message(&player, res);
            }
        }
    });
    b
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
                start_clock(ctx, &game_id);
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

pub fn new_ratings(value: f64, deviation: f64, matches: &Vec<PlayerMatch>) -> GlickoRating {
    let current_rating = GlickoRating { value, deviation };
    let mut results: Vec<GameResult> = vec![];
    for m in matches {
        results.push(GameResult::from(m));
    }
    let new_rating: GlickoRating = new_rating(current_rating.into(), &results, 0.5).into();
    new_rating
}

pub fn start_clock(ctx: Addr<Lobby>, game_id: &String) {
    let game_id = String::from(game_id);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1500));
        loop {
            interval.tick().await;
            let time = ctx.send(GameMessage::time_check(&game_id)).await;
            if let Ok(time) = time {
                if !time {
                    ctx.send(GameMessage::lost_on_time(&game_id)).await;
                    ctx.send(GameMessage::remove_game(&game_id)).await;
                    break;
                }
            } else {
                break;
            }
        }
    });
}

pub async fn save_state(address: Addr<Lobby>) {
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
