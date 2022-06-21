use std::sync::{Arc, RwLock};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::models::redis::{RedisSessions, UserSession};

#[axum_macros::debug_handler]
pub async fn login(r: Extension<rdb>) -> impl IntoResponse {
    let rdb = r.write().unwrap();
    let a = rdb.set(UserSession::new("uros1", true, "abc")).await;
    (StatusCode::CREATED, Json(serde_json::json!({"a": a.id()})))
}

pub async fn callback(Path(id): Path<String>, r: Extension<rdb>) -> impl IntoResponse {
    let rdb = r.read().unwrap();
    let a = rdb.get(&id).await;
    if let Some(s) = a {
        let a = format!("{}", s.id());
        return (StatusCode::CREATED, Json(serde_json::json!({ "a": a })));
    }
    return (StatusCode::CREATED, Json(serde_json::json!({"a": "wrong"})));
}

pub async fn vue_user() -> &'static str {
    "hello vue_user"
}

pub async fn news() -> &'static str {
    "hello news"
}

pub async fn user_games() -> &'static str {
    "hello games"
}

pub type rdb = Arc<RwLock<RedisSessions>>;
