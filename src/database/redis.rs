use async_session::{async_trait, Session};
use axum::{
    extract::{FromRef, FromRequestParts},
    headers::Cookie,
    http::request::Parts,
    http::HeaderValue,
    RequestPartsExt, TypedHeader,
};
use bson::DateTime;
use hyper::{header::SET_COOKIE, HeaderMap, StatusCode};
use mongodb::Collection;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::{arc2, lichess::cookies, AppState};

use super::{mongo::Player, queries::create_player};

pub const AXUM_SESSION_COOKIE_NAME: &str = "axum_session";

/// Struct representing current user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub username: String,
    pub reg: bool,
    pub code_verifier: String,
    pub session: String,
    pub is_new: bool,
    pub watches: Arc<Mutex<String>>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub cookie_value: CookieValue,
}

impl UserSession {
    pub fn new(
        username: &str,
        session: &str,
        reg: bool,
        code_verifier: &str,
        cookie_value: CookieValue,
    ) -> Self {
        Self {
            username: String::from(username),
            reg,
            code_verifier: String::from(code_verifier),
            session: String::from(session),
            is_new: true,
            watches: arc2(String::from("")),
            cookie_value,
        }
    }

    pub fn new_cv(&mut self, code_verifier: &str) {
        self.code_verifier = String::from(code_verifier);
    }

    pub fn new_username(&mut self, username: &str) {
        self.username = String::from(username);
    }

    pub fn new_register(&mut self) {
        self.reg = true;
    }

    pub fn not_new(&mut self) {
        self.is_new = false;
    }

    pub fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if self.is_new {
            let max_age = 60 * 60 * 24 * 365;
            let cookie = format!(
                "{}={}; {} max-age={}; Path=/",
                AXUM_SESSION_COOKIE_NAME,
                &self.session,
                &self.cookie_value.response(),
                max_age
            );
            headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
        }
        headers
    }

    pub fn watch(&self, watching: &String) {
        *self.watches.lock().unwrap() = String::from(watching);
    }
}

#[derive(Clone, Debug)]
pub struct CookieValue {
    pub same_site: String,
    pub secure: String,
    pub http_only: String,
}

impl CookieValue {
    pub fn new(same_site: &str, secure: &str, http_only: &str) -> Self {
        Self {
            same_site: String::from(same_site),
            secure: String::from(secure),
            http_only: String::from(http_only),
        }
    }

    pub fn response(&self) -> String {
        if &self.same_site == "Lax" {
            return format!("SameSite={};", &self.same_site);
        }
        format!(
            "SameSite={}; Secure={}; HttpOnly={};",
            &self.same_site, &self.secure, &self.http_only
        )
    }
}

impl Default for CookieValue {
    fn default() -> Self {
        Self::new("", "", "")
    }
}

/// Redis connection. Used only for saving session.
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

    /// Get session if it exist.
    pub async fn get_session(&mut self, key: &str) -> Option<UserSession> {
        if let Ok(s) = self.con.get::<String, String>(String::from(key)).await {
            if let Ok(value) = serde_json::from_str::<UserSession>(&s) {
                let value = self.set_session(key, value).await;
                return Some(value);
            }
        }
        None
    }

    /// Set new session.
    pub async fn set_session(
        &mut self,
        key: &str,
        mut value: UserSession,
    ) -> UserSession {
        if value.is_new {
            value.not_new();
            self.con
                .set::<String, String, String>(
                    String::from(key),
                    serde_json::to_string(&value).unwrap(),
                )
                .await
                .unwrap();
            let _e = self
                .con
                .expire::<String, usize>(
                    String::from(key),
                    self.ttl_days(value.reg),
                )
                .await;
        }
        value
    }

    /// Create session.
    pub async fn new_session(
        &mut self,
        players: &Collection<Player>,
        cookie_value: CookieValue,
    ) -> UserSession {
        let username = create_player(players).await;
        loop {
            let s = Session::new();
            if (self.get_session(s.id()).await).is_some() {
            } else {
                let value = UserSession::new(
                    &username,
                    s.id(),
                    false,
                    "",
                    cookie_value,
                );
                self.set_session(s.id(), value.clone()).await;
                return value;
            }
        }
    }

    /// Returns one year ttl for registered user.
    pub fn ttl_days(&self, reg: bool) -> usize {
        let day = 60 * 60 * 24;
        if reg {
            return day * 365;
        }
        day * 2
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserSession
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let store = AppState::from_ref(state);
        let cookie: Option<TypedHeader<Cookie>> =
            parts.extract().await.unwrap();

        let mut redis = store.db.redis.clone();
        let prod = store.db.key.prod;
        let cookie_value = cookies(prod);

        let session_cookie = cookie
            .as_ref()
            .and_then(|cookie| cookie.get(AXUM_SESSION_COOKIE_NAME));
        if let Some(session) = session_cookie {
            if let Some(session) = redis.get_session(session).await {
                return Ok(session);
            }
        }
        let session = redis
            .new_session(&store.db.mongo.players, cookie_value)
            .await;
        return Ok(session);
    }
}

/// After login, this struct is returned for updating username on frontend.
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
