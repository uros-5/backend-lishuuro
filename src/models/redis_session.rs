use crate::{
    lichess::login::{create_verifier, random_username},
    models::model::{AppState, User},
};
use actix_session::Session;
use actix_web::web;
use std::sync::Mutex;

pub async fn set_session(session: &Session, code_verifier: String) {
    let result = session.set("codeVerifier", code_verifier);
    match result {
        Err(i) => {}
        _ => (),
    }
}

pub async fn set_username(session: &Session, username: &String) {
    let result = session.set("username", username);
    match result {
        Err(i) => {}
        _ => (),
    }
}

pub async fn set_reg(session: &Session, reg: &bool) {
    let result = session.set("reg", reg);
    match result {
        Err(i) => {}
        _ => (),
    }
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
    let anon = User::new(username.clone());
    let verifier = create_verifier();
    set_session(&session, verifier).await;
    set_username(&session, &anon.username).await;
    set_reg(&session, &false).await;
    let mongo_result = app_data.users.insert_one(&anon, None).await;
    match mongo_result {
        Ok(_) => (),
        Err(e) => {}
    }
    (username, false)
}