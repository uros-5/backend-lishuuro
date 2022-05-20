mod controller;
mod lichess;
mod models;
mod websockets;

use std::sync::Mutex;

use lichess::model::curr_url;
use mongodb::{Collection, Database};
use time::Duration;

use actix_cors::Cors;
use actix_redis::RedisSession;
use actix_web::{web, App, HttpServer};
use controller::{callback, login, news, user_games, vue_user};

use models::model::{AppState, NewsItem};
use mongodb::{options::ClientOptions, Client};

use models::model::{ShuuroGame, User};
use models::p_key::read_key;

use websockets::{lobby::Lobby, start_connection::start_connection};

use actix::prelude::Actor;

use crate::models::db_work::save_state;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017")
        .await
        .expect("No client available");
    client_options.app_name = Some("lishuuro".to_string());
    let client = Client::with_options(client_options).expect("client not found");
    let db = client.database("lishuuro");
    let (users, shuuro_games, news_) = get_cols(&db);
    let lobby = Lobby::new(users, shuuro_games, news_).start();
    let temp_address = lobby.clone();
    let _s = HttpServer::new(move || {
        let (users, shuuro_games, news_) = get_cols(&db);
        let key = read_key();
        App::new()
            .data(Mutex::new(AppState::new(users, news_, shuuro_games, key.1, key.2)))
            .data(lobby.clone())
            .wrap(
                RedisSession::new("127.0.0.1:6379", &key.0)
                    .cookie_max_age(Some(Duration::days(365)))
                    .ttl(172800),
            )
            .wrap(get_cors(key.2))
            .route("/login", web::get().to(login))
            .route("/callback", web::get().to(callback))
            .route("/vue_user", web::get().to(vue_user))
            .route("/news/{id}", web::get().to(news))
            .route("/games/{username}", web::get().to(user_games))
            .service(start_connection)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;
    save_state(temp_address).await;
    Ok(())
}

pub fn get_cors(prod: bool) -> Cors {
    let addr: &str = curr_url(prod).1; 
    let cors = Cors::default()
        .allow_any_header()
        .allowed_origin(addr)
        .allow_any_method()
        .supports_credentials();
    cors
}

pub fn get_cols(
    db: &Database,
) -> (
    Collection<User>,
    Collection<ShuuroGame>,
    Collection<NewsItem>,
) {
    let users = db.collection::<User>("users");
    let shuuro_games = db.collection::<ShuuroGame>("shuuroGames");
    let news = db.collection::<NewsItem>("news");

    (users, shuuro_games, news)
}
