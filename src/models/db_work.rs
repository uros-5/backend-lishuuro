use crate::websockets::lobby::Lobby;
use crate::websockets::messages::{GameMessage, GameMessageType};
use actix::prelude::Context;
use actix::AsyncContext;
use bson::{doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use futures::Future;
use glicko2::{new_rating, GameResult, GlickoRating};
use mongodb::Collection;
use std::collections::HashMap;
use std::str::FromStr;

use super::model::{ActivePlayer, NewsItem, PlayerMatch, ShuuroGame, User};

macro_rules! gr {
    ($l:expr, $res:expr) => {
        GameResult::from(&PlayerMatch::new($l, $res))
    };
}

macro_rules! gr2 {
    ($result:expr, $wr:expr, $br:expr) => {{
        let wr_n: GameResult;
        let br_n: GameResult;
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
                    fn white_win() {}
                    let wr = game.ratings.get(&game.white).unwrap();
                    let br = game.ratings.get(&game.white).unwrap();

                    let mut wr_new: GameResult;
                    let mut br_new: GameResult;

                    //let a = gr!(wr, "d");
                    if [3, 4, 5, 6].contains(&game.status) {
                        wr_new = gr!(wr, "d");
                        br_new = gr!(br, "d");
                    } else if game.status == 1 {
                        (wr_new, br_new) = gr2!(&game.result, wr, br);
                    } else if game.status == 7 {
                        (wr_new, br_new) = gr2!(&game.result, wr, br);
                    }
                }

                // update user ratings for both
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

pub fn update_all(s: &Lobby, all: Vec<(String, ShuuroGame)>) -> impl Future<Output = ()> {
    let s = s.clone();
    let all = all.clone();
    let b = Box::pin(async move {
        for i in all {
            let filter = doc! {"_id": ObjectId::from_str(i.0.as_str()).unwrap()};
            let update = doc! {"$set": bson::to_bson(&i.1).unwrap()};
            let game1 = s.db_shuuro_games.find_one_and_update(filter, update, None);
            match game1.await {
                _g => {}
            };
        }
    });
    b
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
            ctx2.do_send(GameMessage {
                message_type: GameMessageType::news(active_player, news_),
            });
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
                let game_id = id.replace("ObjectId(\"", "").replace("\")", "");
                ctx.do_send(GameMessage {
                    message_type: GameMessageType::new_adding_game(
                        game_id,
                        users.clone(),
                        shuuro_game,
                    ),
                });
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

pub fn new_ratings(value: f64, deviation: f64, matches: Vec<PlayerMatch>) -> GlickoRating {
    let current_rating = GlickoRating { value, deviation };
    let mut results: Vec<GameResult> = vec![];
    for m in matches {
        results.push(GameResult::from(&m));
    }
    let new_rating: GlickoRating = new_rating(current_rating.into(), &results, 0.5).into();
    new_rating
}
