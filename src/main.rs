mod controller;
mod lichess;
mod models;
mod websockets;

use std::sync::Mutex;

use time::Duration;

use actix_cors::Cors;
use actix_redis::RedisSession;
use actix_web::{web, App, HttpServer};
use controller::{callback, login, news, test, vue_user};

use models::model::{AppState, NewsItem};
use mongodb::{options::ClientOptions, Client};

use models::model::{ShuuroGame, User};
use websockets::{lobby::Lobby, start_connection::start_connection};

use actix::prelude::Actor;

const PRIVATE_KEY: [u8; 32] = [
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
];

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("http://localhost:8080/test");
    let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017")
        .await
        .expect("No client available");
    let client = Client::with_options(client_options.clone()).expect("client not found");
    client_options.app_name = Some("lishuuro".to_string());
    let db = client.database("lishuuro");
    let users = db.collection::<User>("users");
    let shuuro_games = db.collection::<ShuuroGame>("shuuroGames");
    let news = db.collection::<NewsItem>("news");
    let lobby = Lobby::new(users, shuuro_games, news).start();
    HttpServer::new(move || {
        let users = db.collection::<User>("users");
        let news_items = db.collection::<NewsItem>("news");
        App::new()
            .data(Mutex::new(AppState::new(users, news_items)))
            .data(lobby.clone())
            .wrap(
                RedisSession::new("127.0.0.1:6379", &PRIVATE_KEY)
                    .cookie_max_age(Some(Duration::days(365))),
            )
            .wrap(get_cors())
            .route("/login", web::get().to(login))
            .route("/callback", web::get().to(callback))
            .route("/vue_user", web::get().to(vue_user))
            .route("/test", web::get().to(test))
            .route("/news/{id}", web::get().to(news))
            .service(start_connection)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

pub fn get_cors() -> Cors {
    let cors = Cors::default()
        .allow_any_header()
        .allow_any_origin()
        .allow_any_method()
        .supports_credentials();
    cors
}
