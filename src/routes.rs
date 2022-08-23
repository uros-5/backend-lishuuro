use std::{collections::HashMap, sync::Arc};

use axum::{extract::Query, response::Redirect, Extension, Json};
use hyper::HeaderMap;

use crate::{
    database::{
        queries::player_exist,
        redis::{UserSession, VueUser},
        Database,
    },
    lichess::{
        curr_url,
        login::{get_lichess_token, get_lichess_user, login_url},
    },
};

pub async fn login(mut user: UserSession, Extension(db): Extension<Arc<Database>>) -> Redirect {
    let key = &db.key;
    let mut redis = db.redis.clone();
    let url = login_url(&key.login_state, key.prod);
    user.new_cv(&url.1.as_str());
    redis.set_session(&user.session, user.clone()).await;
    Redirect::permanent(url.0.as_str())
}
pub async fn callback(
    Query(params): Query<HashMap<String, String>>,
    Extension(db): Extension<Arc<Database>>,
    user: UserSession,
) -> Redirect {
    let key = &db.key;
    let mongo = &db.mongo;
    let mut redis = db.redis.clone();
    let r = curr_url(key.prod);
    let r = format!("{}/logged", r.1);
    if let Some(code) = params.get(&String::from("code")) {
        let lichess_token = get_lichess_token(code, &user.code_verifier, key.prod).await;
        if lichess_token.access_token != "" {
            let lichess_user = get_lichess_user(lichess_token.access_token).await;
            if lichess_user != "" {
                let player = player_exist(&mongo.players, &lichess_user, &user).await;
                if let Some(player) = player {
                    let session = &player.session.clone();
                    redis.set_session(session, player).await;
                }
            }
        }
    }
    Redirect::permanent(r.as_str())
}

pub async fn vue_user(user: UserSession) -> (HeaderMap, Json<VueUser>) {
    let headers = user.headers();
    (headers, Json(VueUser::from(&user)))
}
