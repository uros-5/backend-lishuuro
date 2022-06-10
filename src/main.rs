mod database;
mod lichess;
mod websockets;
mod controller;

use axum::{
    routing::{get, post},
    extract::{Extension},
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
};
use std::sync::{Arc,RwLock};
use std::net::SocketAddr;
use controller::*;


#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let mut counter: Arc<RwLock<i32>> = Arc::new(RwLock::new(0));
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/login", get(login))
        // `POST /users` goes to `create_user`
        .route("/callback", get(callback))
        .route("/vue_user", get(vue_user))
        .route("/news", get(news))
        .layer(Extension(counter));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
