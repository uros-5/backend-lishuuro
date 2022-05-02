use crate::websockets::lobby::Lobby;
use crate::websockets::messages::{GameMessage, GameMessageType};
use actix::prelude::{Context};
use actix::AsyncContext;
use bson::{doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use futures::Future;
use mongodb::Collection;
use std::str::FromStr;

use super::model::{ActivePlayer, NewsItem, ShuuroGame};

pub fn update_entire_game(s: &Lobby, id: &String, game: &ShuuroGame) -> impl Future<Output = ()> {
    let filter = doc! {"_id": ObjectId::from_str(id.as_str()).unwrap()};
    let update = doc! {"$set": bson::to_bson(&game).unwrap()};
    let self2 = s.clone();
    let b = Box::pin(async move {
        let game1 = self2
            .db_shuuro_games
            .find_one_and_update(filter, update, None);
        match game1.await {
            _g => {}
        };
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
    game_id: &'a String,
    player: &'a ActivePlayer,
) -> impl Future<Output = ()> + 'a {
    let filter = doc! {"_id": ObjectId::from_str(game_id).unwrap()};
    let db = self1.db_shuuro_games.clone();
    let self2 = self1.clone();

    let b = Box::pin(async move {
        let game = db.find_one(filter, None);
        if let Ok(g) = game.await {
            if let Some(g) = g {
                let res = serde_json::json!({"t": "live_game_start", "game_id": game_id, "game_info": &g.clone()});
                self2.send_message(player, res);
            }
        }
    });
    b
}

pub fn new_game<'a>(
    ctx: &Context<Lobby>,
    col: Collection<ShuuroGame>,
    users: [String; 2],
    shuuro_game: ShuuroGame,
) -> impl Future<Output = ()> + 'a {
    let ctx = ctx.address().clone();
    let shuuro_game = shuuro_game.clone();
    let b = Box::pin(async move {
        let game_added = col.insert_one(&shuuro_game, None);
        match game_added.await {
            g => {
                let id = g.ok().unwrap().inserted_id.to_string();
                let game_id = id.replace("ObjectId(\"", "").replace("\")", "");
                ctx.do_send(GameMessage {
                    message_type: GameMessageType::new_adding_game(game_id, users, shuuro_game),
                });
            }
        }
    });
    b
}
