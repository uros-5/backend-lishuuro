use axum::{http::HeaderValue, routing::get, Extension, Router};
use hyper::{header::SET_COOKIE, HeaderMap};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::{Any, Cors, CorsLayer};

mod database;
mod lichess;
mod routes;

use database::redis::RedisCli;
use database::Database;
use lichess::{curr_url, MyKey};
use routes::{callback, login, vue_user};

#[tokio::main]
async fn main() {
    // build our application with a route
    let redis = Arc::new(RedisCli::default());
    let mongo = Arc::new(Database::new().await);
    let my_key = Arc::new(MyKey::default());
    let app = Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/vue_user", get(vue_user))
        .layer(Extension(redis))
        .layer(Extension(mongo))
        .layer(cors(&my_key))
        .layer(Extension(my_key));

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
    let mut cors = CorsLayer::new();
    cors.allow_origin(addr.1.parse::<HeaderValue>().unwrap())
        .allow_credentials(true)
}
