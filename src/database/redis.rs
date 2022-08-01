use async_session::{async_trait, Session};
use axum::{
    extract::{Extension, FromRequest, RequestParts},
    headers::Cookie,
    http::HeaderValue,
    TypedHeader,
};
use bson::DateTime;
use hyper::{header::SET_COOKIE, HeaderMap, StatusCode};
use mongodb::Collection;
use redis::{Client, Cmd, Commands};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{sync::Arc, time::Duration};

use crate::lichess::login::random_username;
use crate::lichess::login_helpers::create_verifier;

use super::{queries::create_player, Database, Player};

pub const AXUM_SESSION_COOKIE_NAME: &str = "axum_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub username: String,
    pub reg: bool,
    pub code_verifier: String,
    pub session: String,
    pub is_new: bool,
}

impl UserSession {
    pub fn new(username: &str, session: &str, reg: bool, code_verifier: &str) -> Self {
        Self {
            username: String::from(username),
            reg,
            code_verifier: String::from(code_verifier),
            session: String::from(session),
            is_new: true,
        }
    }

    pub fn update(&mut self, code_verifier: &str) {
        self.code_verifier = String::from(code_verifier);
    }

    pub fn register(&mut self) {
        self.reg = true;
    }

    pub fn username(&mut self, username: &str) {
        self.username = String::from(username);
    }

    pub fn not_new(&mut self) {
        self.is_new = false;
    }

    pub fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if self.is_new {
            let cookie = format!("{}={}", AXUM_SESSION_COOKIE_NAME, &self.session);
            headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
        }
        headers
    }
}

#[derive(Debug, Clone)]
pub struct RedisCli {
    cli: Client,
}

impl Default for RedisCli {
    fn default() -> Self {
        let cli = Client::open("redis://127.0.0.1/").unwrap();
        Self { cli }
    }
}

impl RedisCli {
    pub async fn get_session(&self, key: &str) -> Option<UserSession> {
        if let Ok(mut con) = self.cli.get_connection() {
            if let Ok(s) = con.get::<String, String>(String::from(key)) {
                if let Ok(mut value) = serde_json::from_str::<UserSession>(&s) {
                    value.not_new();
                    return Some(value);
                }
            }
        }
        None
    }

    pub async fn set_session(&self, key: &str, value: UserSession) {
        if let Ok(mut con) = self.cli.get_connection() {
            con.set::<String, String, String>(
                String::from(key),
                serde_json::to_string(&value).unwrap(),
            )
            .unwrap();
            con.expire::<String, usize>(String::from(key), self.ttl_days(value.reg));
        }
    }

    pub async fn new_session(&self, players: &Collection<Player>) -> UserSession {
        let username = create_player(players).await;
        loop {
            let s = Session::new();
            if let Some(s) = self.get_session(s.id()).await {
            } else {
                let value = UserSession::new(&username, s.id(), false, "");
                self.set_session(s.id(), value.clone()).await;
                return value;
            }
        }
    }

    pub fn ttl_days(&self, reg: bool) -> usize {
        let day = 60 * 60 * 24;
        if reg {
            return day * 365;
        }
        day * 2
    }
}

#[async_trait]
impl<B> FromRequest<B> for UserSession
where
    B: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(redis) = Extension::<Arc<RedisCli>>::from_request(req)
            .await
            .expect("redis missing");

        let Extension(db) = Extension::<Arc<Database>>::from_request(req)
            .await
            .expect("database missing");

        let cookie = Option::<TypedHeader<Cookie>>::from_request(req)
            .await
            .unwrap();

        let session_cookie = cookie
            .as_ref()
            .and_then(|cookie| cookie.get(AXUM_SESSION_COOKIE_NAME));
        if !session_cookie.is_none() {
            if let Some(session) = redis.get_session(session_cookie.unwrap()).await {
                return Ok(session);
            }
        }
        let session = redis.new_session(&db.players).await;
        return Ok(session);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VueUser {
    pub username: String,
    pub logged: bool,
}

impl From<&UserSession> for VueUser {
    fn from(user: &UserSession) -> Self {
        Self {
            username: String::from(&user.username),
            logged: user.reg,
        }
    }
}

impl From<&UserSession> for UserSession {
    fn from(other: &UserSession) -> Self {
        Self {
            username: String::from(&other.username),
            reg: other.reg,
            code_verifier: String::from(&other.code_verifier),
            session: String::from(&other.code_verifier),
            is_new: other.is_new,
        }
    }
}

impl From<&UserSession> for Player {
    fn from(other: &UserSession) -> Self {
        Player {
            _id: String::from(&other.username),
            reg: other.reg,
            created_at: DateTime::now(),
        }
    }
}
