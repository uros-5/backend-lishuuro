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
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{mongo::Mongo, mongo::Player, queries::create_player, Database};

pub const AXUM_SESSION_COOKIE_NAME: &str = "axum_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub username: String,
    pub reg: bool,
    pub code_verifier: String,
    pub session: String,
    pub is_new: bool,
    pub watches: String
}

impl UserSession {
    pub fn new(username: &str, session: &str, reg: bool, code_verifier: &str) -> Self {
        Self {
            username: String::from(username),
            reg,
            code_verifier: String::from(code_verifier),
            session: String::from(session),
            is_new: true,
            watches: String::from("")
        }
    }

    pub fn new_cv(&mut self, code_verifier: &str) {
        self.code_verifier = String::from(code_verifier);
    }

    pub fn new_username(&mut self, username: &str) {
        self.username = String::from(username);
    }

    pub fn register(&mut self) {
        self.reg = true;
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

#[derive(Clone)]
pub struct RedisCli {
    con: ConnectionManager,
}

impl RedisCli {
    pub async fn default() -> Self {
        let cli = Client::open("redis://127.0.0.1/").unwrap();
        let con = ConnectionManager::new(cli).await.unwrap();
        Self { con }
    }

    pub async fn get_session(&mut self, key: &str) -> Option<UserSession> {
        if let Ok(s) = self.con.get::<String, String>(String::from(key)).await {
            if let Ok(mut value) = serde_json::from_str::<UserSession>(&s) {
                value.not_new();
                self.set_session(key, value.clone()).await;
                return Some(value);
            }
        }
        None
    }

    pub async fn set_session(&mut self, key: &str, value: UserSession) {
        self.con
            .set::<String, String, String>(
                String::from(key),
                serde_json::to_string(&value).unwrap(),
            )
            .await
            .unwrap();
        let e = self
            .con
            .expire::<String, usize>(String::from(key), self.ttl_days(value.reg))
            .await;
    }

    pub async fn new_session(&mut self, players: &Collection<Player>) -> UserSession {
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
        let Extension(mut db) = Extension::<Arc<Database>>::from_request(req)
            .await
            .expect("db is missing");
        let cookie = Option::<TypedHeader<Cookie>>::from_request(req)
            .await
            .unwrap();

        let mut redis = db.redis.clone();

        let session_cookie = cookie
            .as_ref()
            .and_then(|cookie| cookie.get(AXUM_SESSION_COOKIE_NAME));
        if !session_cookie.is_none() {
            if let Some(session) = redis.get_session(session_cookie.unwrap()).await {
                return Ok(session);
            }
        }
        let session = redis.new_session(&db.mongo.players).await;
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

impl From<&UserSession> for Player {
    fn from(other: &UserSession) -> Self {
        Player {
            _id: String::from(&other.username),
            reg: other.reg,
            created_at: DateTime::now(),
        }
    }
}
