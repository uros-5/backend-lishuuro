use axum::{http::HeaderValue, routing::get, Router};

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::Mutex as Mutex2;
use tower_http::cors::CorsLayer;

mod database;
mod lichess;
mod nuxt;
mod routes;
mod websockets;

use lichess::{curr_url, MyKey};
use nuxt::nuxt;
use routes::{article, callback, get_games, login, vue_user};

use crate::{
    database::Database,
    websockets::{websocket_handler, WsState},
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let db = Database::new().await;
    let cors_layer = cors(&db.key);
    let db = Arc::new(db);
    let ws = Arc::new(WsState::default());
    ws.load_unfinished(&db.mongo.games).await;
    let state = AppState::new(db, ws);
    let app = Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/vue_user", get(vue_user))
        .route("/ws/", get(websocket_handler))
        .route("/news/:id", get(article))
        .route("/games/:username/:page", get(get_games))
        .nest("/nuxt", nuxt())
        .with_state(state)
        .layer(cors_layer);
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub ws: Arc<WsState>,
}

impl AppState {
    pub fn new(db: Arc<Database>, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
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
