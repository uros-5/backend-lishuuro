use std::{collections::HashMap, sync::Arc};

use axum::{extract::Query, http::HeaderValue, response::Redirect, Extension, Json};
use hyper::{header::SET_COOKIE, HeaderMap};
use querystring::querify;

use crate::{
    database::{
        queries::player_exist,
        redis::{RedisCli, UserSession, VueUser, AXUM_SESSION_COOKIE_NAME},
        Database,
    },
    lichess::{
        curr_url,
        login::{get_lichess_token, get_lichess_user, login_url},
        MyKey,
    },
};

pub async fn login(
    mut user: UserSession,
    redis: Extension<Arc<RedisCli>>,
    Extension(key): Extension<Arc<MyKey>>,
) -> Redirect {
    let url = login_url(&key.login_state, key.prod);
    user.update(&url.1.as_str());
    redis.set_session(&user.code_verifier, user.clone());
    Redirect::permanent(url.0.as_str())
}
pub async fn callback(
    Query(params): Query<HashMap<String, String>>,
    Extension(db): Extension<Arc<Database>>,
    Extension(key): Extension<Arc<MyKey>>,
    Extension(redis): Extension<Arc<RedisCli>>,
    user: UserSession,
) {
    let r = curr_url(key.prod);
    if let Some(code) = params.get(&String::from("code")) {
        let lichess_token = get_lichess_token(code, &user.code_verifier, key.prod).await;
        if lichess_token.access_token != "" {
            let lichess_user = get_lichess_user(lichess_token.access_token).await;
            if lichess_user != "" {
                let player = player_exist(&db.players, &lichess_user, &user).await;
                if let Some(player) = player {
                    let r = format!("{}/logged", r.1);
                    let session = &player.session.clone();
                    redis.set_session(session, player).await;
                }
            }
        }
    }
}

pub async fn vue_user(user: UserSession) -> (HeaderMap, Json<VueUser>) {
    let headers = user.headers();
    (headers, Json(VueUser::from(&user)))
}
