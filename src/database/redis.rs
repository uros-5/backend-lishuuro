use async_session::async_trait;
use axum::{
    extract::{Extension, FromRequest, RequestParts},
    headers::Cookie,
    TypedHeader,
};
use hyper::StatusCode;
use redis::{Client, Cmd, Commands};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

use crate::lichess::login::random_username;

const AXUM_SESSION_COOKIE_NAME: &str = "axum_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub username: String,
    pub reg: bool,
    pub code_verifier: String,
}

impl UserSession {
    pub fn new(username: &str, reg: bool, code_verifier: &str) -> Self {
        Self {
            username: String::from(username),
            reg,
            code_verifier: String::from(code_verifier),
        }
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
    pub async fn get_session(&self, key: String) -> Option<UserSession> {
        if let Ok(mut con) = self.cli.get_connection() {
            if let Ok(s) = con.get::<String, String>(key) {
                if let Ok(value) = serde_json::from_str::<UserSession>(&s) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub async fn set_session(&self, key: String, value: UserSession) {
        if let Ok(mut con) = self.cli.get_connection() {
            con.set::<String, String, String>(key, serde_json::to_string(&value).unwrap())
                .unwrap();
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for UserSession
where
    B: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(redis) = Extension::<RedisCli>::from_request(req)
            .await
            .expect("redis missing");

        let cookie = Option::<TypedHeader<Cookie>>::from_request(req)
            .await
            .unwrap();

        let session_cookie = cookie
            .as_ref()
            .and_then(|cookie| cookie.get(AXUM_SESSION_COOKIE_NAME));
        if session_cookie.is_none() {

            // napravi username
            // vidi da li postoji kod mongodb, dokle kod nema
            // ako nema dodaj u mongodb
            // napravi session id i vidi da li ima u redisu
            // ako ima napravi novu sessiju
            // ako nema dodaj sa sve imenom itd
        }

        Err(Rejection)
    }
}
