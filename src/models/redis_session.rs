use crate::{
    lichess::login::{create_verifier, random_username},
    models::model::{AppState, User},
};
use actix_session::Session;
use actix_web::web;
use serde::Serialize;
use std::sync::Mutex;

pub async fn set_value<T: Serialize>(session: &Session, key: &str, value: &T) {
    if let Err(_e) = session.set(key, value) {}
}

pub async fn is_logged(session: &Session) -> (bool, String) {
    let result = session.get::<String>("username");
    match result {
        Ok(i) => match i {
            Some(s) => {
                return (session.get::<bool>("reg").unwrap().unwrap(), s);
            }
            None => {
                return (false, String::from(""));
            }
        },
        Err(_) => (false, String::from("")),
    }
}

pub async fn new_user(session: &Session, app_data: web::Data<Mutex<AppState>>) -> (String, bool) {
    let app_data = app_data.lock().unwrap();
    let username = random_username();
    let anon = User::new(&username);
    let verifier = create_verifier();
    set_value(&session, "codeVerifier", &verifier).await;
    set_value(&session, "username", &anon.username).await;
    set_value(&session, "reg", &false).await;
    let mongo_result = app_data.users.insert_one(&anon, None).await;
    match mongo_result {
        Ok(_) => (),
        Err(_) => {}
    }
    (username, false)
}
