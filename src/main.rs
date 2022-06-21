mod controller;
mod lichess;
mod models;

use controller::*;

use axum::{routing::get, Extension, Router};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use crate::controller::rdb;
use crate::models::redis::RedisSessions;

#[tokio::main]
async fn main() {
    // build our application with a route
    let redis_store = RedisSessions::new().await;
    let redis_store = RwLock::new(redis_store);
    let redis_store: rdb = Arc::new(redis_store);
    let app = Router::new()
        .route("/login", get(login))
        .route("/callback/:id", get(callback))
        .route("/vue_user", get(vue_user))
        .route("/user_games", get(user_games))
        .route("/news", get(news))
        .layer(Extension(redis_store));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
