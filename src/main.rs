mod controller;
mod models;
mod websockets;
mod lichess;

use std::sync::Mutex;

use time::Duration;

use actix_cors::Cors;
use actix_redis::RedisSession;
use actix_web::{web, App, HttpServer};
use controller::{callback, login, vue_user};

use models::model::AppState;
use mongodb::{options::ClientOptions, Client};

use models::model::{ShuuroGame, User};
use websockets::{lobby::Lobby, start_connection::start_connection};

use actix::prelude::Actor;

const PRIVATE_KEY: [u8; 32] = [
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5
];

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017")
        .await
        .expect("No client available");
    let client = Client::with_options(client_options.clone()).expect("client not found");
    client_options.app_name = Some("lishuuro".to_string());
    let db = client.database("lishuuro");
    let users = db.collection::<User>("users");
    let shuuro_games = db.collection::<ShuuroGame>("shuuroGames");
    let lobby = Lobby::new(users, shuuro_games).start();
    HttpServer::new(move || {
        let users = db.collection::<User>("users");
        App::new()
            .data(Mutex::new(AppState::new(users)))
            .data(lobby.clone())
            .wrap(
                RedisSession::new("127.0.0.1:6379", &PRIVATE_KEY)
                    .cookie_max_age(Some(Duration::days(365))),
            )
            .wrap(get_cors())
            .route("/login", web::get().to(login))
            .route("/callback", web::get().to(callback))
            .route("/vue_user", web::get().to(vue_user))
            .service(start_connection)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

pub fn get_cors() -> Cors {
    Cors::permissive()
}
