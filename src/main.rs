use axum::{http::HeaderValue, routing::get, Extension, Router};

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::Mutex as Mutex2;
use tower_http::cors::CorsLayer;

mod database;
mod lichess;
mod routes;
mod websockets;

use lichess::{curr_url, MyKey};
use routes::{article, callback, get_games, login, vue_user};

use crate::{
    database::Database,
    websockets::{websocket_handler, WsState},
};

#[tokio::main]
async fn main() {
    // build our application with a route
    let db = Database::new().await;
    let cors_layer = cors(&db.key);
    let db = Arc::new(db);
    let games = db.mongo.games.clone();
    let ws = Arc::new(WsState::default());
    ws.load_unfinished(&db.mongo.games).await;
    let app = Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/vue_user", get(vue_user))
        .route("/ws/", get(websocket_handler))
        .route("/news/:id", get(article))
        .route("/games/:username", get(get_games))
        .layer(Extension(db))
        .layer(Extension(ws))
        .layer(cors_layer);

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn cors(key: &MyKey) -> CorsLayer {
    let addr = curr_url(key.prod);
    let cors = CorsLayer::new();
    cors.allow_origin(addr.1.parse::<HeaderValue>().unwrap())
        .allow_credentials(true)
}

pub fn arc2<T>(data: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(data))
}

pub fn arc3<T>(data: T) -> Arc<Mutex2<T>> {
    Arc::new(Mutex2::new(data))
}
