use axum::{http::HeaderValue, routing::get, Extension, Router};
use hyper::{header::SET_COOKIE, HeaderMap};
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

mod database;
mod lichess;
mod routes;

use database::redis::RedisCli;

#[tokio::main]
async fn main() {
    // build our application with a route
    let redis = RedisCli::default();
    let app = Router::new()
        .route("/s", get(nesto))
        .layer(Extension(redis));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn nesto() {}
