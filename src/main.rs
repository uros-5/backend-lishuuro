use axum::{http::HeaderValue, routing::get, Extension, Router};
use std::{
    net::SocketAddr,
    sync::{Arc},
};
use tower_http::cors::CorsLayer;

mod database;
mod lichess;
mod routes;
mod websockets;

use lichess::{curr_url, MyKey};
use routes::{callback, login, vue_user};

use crate::{database::Database, websockets::websocket_handler};

#[tokio::main]
async fn main() {
    // build our application with a route
    let db = Database::new().await;
    let cors_layer = cors(&db.key);
    let db = Arc::new(db);
    let app = Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/vue_user", get(vue_user))
        .route("/ws/", get(websocket_handler))
        .layer(Extension(db))
        .layer(cors_layer);

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("listening on http://{}/login", addr);
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
