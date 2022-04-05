use std::{sync::Mutex, time::Duration};

use actix_session::Session;
use actix_web::{http, web, HttpRequest, HttpResponse, Responder};
use bson::doc;
use itertools::Itertools;
use querystring::querify;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;

use crate::{
    lichess::login::*,
    models::model::{AppState, User},
    models::redis_session::*,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Info {
    counter: u8,
}

pub async fn login(_: HttpRequest, session: Session) -> impl Responder {
    let (lichess_url, verifier) = login_url();
    set_session(&session, verifier).await;
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
                                set_username(&session, &lichess_user).await;
                                set_reg(&session, &true).await;
                                let new_user = User::new(lichess_user);
                                app_data.users.insert_one(new_user, None).await;
                            }
                            _ => {
                                set_username(&session, &lichess_user).await;
                                set_reg(&session, &true).await;
                            }
                        },
                        Err(err) => {}
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

pub async fn test(session: Session) -> impl Responder {
    let now = OffsetDateTime::now_utc();
    web::Json(json!({ "d": now.to_string()}))
}
