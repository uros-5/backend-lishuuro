use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    Json,
};
use hyper::HeaderMap;
use serde_json::Value;

use crate::{
    database::{
        queries::{get_article, get_player_games, player_exist},
        redis::{UserSession, VueUser},
    },
    lichess::{
        curr_url,
        login::{get_lichess_token, get_lichess_user, login_url},
    },
    AppState,
};

/// Route for lichess login.
pub async fn login(
    mut user: UserSession,
    State(state): State<AppState>,
) -> Redirect {
    let key = &state.db.key;
    let mut redis = state.db.redis.clone();
    let url = login_url(&key.login_state, key.prod);
    user.new_cv(&url.1);
    redis.set_session(&user.session, user.clone(), true).await;
    Redirect::permanent(url.0.as_str())
}

/// Callback after successfull login.
pub async fn callback(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
    user: UserSession,
) -> Redirect {
    let key = &state.db.key;
    let mongo = &state.db.mongo;
    let mut redis = state.db.redis.clone();
    let r = curr_url(key.prod);
    let r = format!("{}/logged", r.1);
    if let Some(code) = params.get(&String::from("code")) {
        let lichess_token =
            get_lichess_token(code, &user.code_verifier, key.prod).await;
        if !lichess_token.access_token.is_empty() {
            let lichess_user =
                get_lichess_user(lichess_token.access_token).await;
            if !lichess_user.is_empty() {
                let player =
                    player_exist(&mongo.players, &lichess_user, &user).await;
                if let Some(player) = player {
                    let session = String::from(&player.session);
                    redis.set_session(&session, player, true).await;
                }
            }
        }
    }
    Redirect::permanent(r.as_str())
}

/// Getting username for current session.
pub async fn vue_user(user: UserSession) -> (HeaderMap, Json<VueUser>) {
    let headers = user.headers();
    (headers, Json(VueUser::from(&user)))
}

/// Get last 5 games for selected player.
pub async fn get_games(
    Path((username, page)): Path<(String, u64)>,
    State(state): State<AppState>,
) -> Json<Value> {
    if let Some(games) =
        get_player_games(&state.db.mongo.games, &username, page).await
    {
        return Json(serde_json::json!({"exist": true, "games": games}));
    }
    Json(serde_json::json!({"exist": false}))
}

/// Get article.
pub async fn article(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<Value> {
    if let Some(article) = get_article(&state.db.mongo.articles, &id).await {
        return Json(serde_json::json!({"exist": true, "news": article}));
    }
    Json(serde_json::json!({"exist": false}))
}
