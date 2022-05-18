use std::sync::Mutex;

use actix_session::Session;
use actix_web::{http, web, HttpRequest, HttpResponse, Responder};
use bson::doc;
use futures::TryStreamExt;
use mongodb::options::FindOptions;
use querystring::querify;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;

use crate::{
    lichess::login::*,
    models::model::{AppState, ShuuroGame, User},
    models::redis_session::*,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Info {
    counter: u8,
}

pub async fn login(
    _: HttpRequest,
    session: Session,
    app_data: web::Data<Mutex<AppState>>,
) -> impl Responder {
    let login_state = &app_data.lock().unwrap().login_state;
    let (lichess_url, verifier) = login_url(login_state);
    set_value(&session, "codeVerifier", &verifier).await;
    HttpResponse::Found()
        .header(http::header::LOCATION, lichess_url.to_string())
        .finish()
        .into_body()
}

pub async fn callback<'a>(
    req: HttpRequest,
    session: Session,
    app_data: web::Data<Mutex<AppState>>,
) -> impl Responder {
    let app_data = app_data.lock().unwrap();
    let query_code = querify(req.query_string());
    if query_code.len() > 0 {
        for i in query_code {
            let lichess_token = get_lichess_token(&session, i.1).await;
            if lichess_token.access_token != "" {
                let lichess_user = get_lichess_user(lichess_token.access_token).await;
                if lichess_user != "" {
                    let user_exist = app_data
                        .users
                        .find_one(doc! {"_id": &lichess_user}, None)
                        .await;
                    match user_exist {
                        Ok(user) => match user {
                            None => {
                                set_value(&session, "username", &lichess_user).await;
                                set_value(&session, "reg", &true).await;
                                let new_user = User::new(&lichess_user);
                                #[warn(unused_must_use)]
                                app_data.users.insert_one(new_user, None).await;
                            }
                            _ => {
                                set_value(&session, "username", &lichess_user).await;
                                set_value(&session, "reg", &true).await;
                            }
                        },
                        Err(_) => {}
                    }
                }
            }
            break;
        }
    }
    HttpResponse::Found()
        .header(http::header::LOCATION, "http://localhost:3000/logged")
        .finish()
        .into_body()
}

pub async fn vue_user(_session: Session) -> impl Responder {
    let (logged, username) = is_logged(&_session).await;
    web::Json(json!( { "logged": logged, "username": username } ))
}

pub async fn news(
    _session: Session,
    app_data: web::Data<Mutex<AppState>>,
    path: web::Path<String>,
) -> impl Responder {
    let app_data = app_data.lock().unwrap();
    let item = app_data
        .news
        .find_one(doc! {"title": path.as_str()}, None)
        .await;
    match item {
        Ok(i) => {
            if let Some(news) = i {
                return web::Json(json!({"exist": true, "news": news}));
            }
        }
        Err(_e) => {}
    }
    web::Json(json!({"exist": false}))
}

pub async fn user_games(
    app_data: web::Data<Mutex<AppState>>,
    path: web::Path<String>,
) -> impl Responder {
    let app_data = app_data.lock().unwrap();
    let find_options = FindOptions::builder()
        .sort(doc! {"$natural": -1})
        .limit(Some(5))
        .build();
    let item = app_data
        .games
        .find(
            doc! {"$or": [{"white": path.as_str()}, {"black": path.as_str()}]},
            Some(find_options),
        )
        .await;
    match item {
        Ok(c) => {
            let games: Vec<ShuuroGame> = c.try_collect().await.unwrap_or_else(|_| vec![]);
            return web::Json(json!({"exist": true, "games": games}));
        }
        Err(_e) => {}
    }
    web::Json(json!({"exist": false}))
}