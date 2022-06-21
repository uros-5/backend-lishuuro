use std::time::Duration;

use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    handler::Handler,
    response::IntoResponse,
    Extension,
};
use redis::{Client, Cmd, Commands};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

#[derive(Clone)]
pub struct RedisSessions {
    store: MemoryStore,
    client: Client,
}

#[async_trait]
impl<B> FromRequest<B> for RedisSessions
where
    B: Send,
{
    type Rejection = ();

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(store) = Extension::<RedisSessions>::from_request(req)
            .await
            .expect("`MemoryStore` extension missing");
        Ok(store)
    }
}

impl RedisSessions {
    pub async fn new() -> Self {
        let store = MemoryStore::new();
        let client = Client::open("redis://127.0.0.1/").ok().unwrap();
        let mut rs = Self { store, client };
        rs.set_all();
        rs
    }

    async fn set_all(&mut self) {
        let mut con = self.client.get_connection().ok().unwrap();
        let all: Vec<String> = con.scan::<String>().unwrap().collect();
        for k in all {
            if let Ok(v) = con.get::<&String, String>(&k) {
                if let Ok(ttl) = con.ttl::<&String, u64>(&k) {
                    if let Ok(s) = serde_json::from_str::<UserSession>(&v) {
                        let mut session = Session::new();
                        let _ = session.insert(&k, s);
                        let _ = session.expire_in(Duration::from_secs(ttl));
                        self.store.store_session(session).await;
                    }
                }
            }
        }
    }

    pub async fn set(&self, value: UserSession) -> Session {
        let mut session = Session::new();
        let _ = session.insert("s", value.clone());
        let converted = serde_json::json!(value);
        let ttl = self.ttl(value.reg);
        Cmd::set(session.id(), converted.as_str().unwrap());
        Cmd::expire(session.id(), ttl as usize);
        let _ = session.expire_in(Duration::from_secs(ttl));
        self.store.store_session(session.clone()).await;
        session
    }

    pub async fn get(&self, id: &str) -> Option<Session> {
        if let Ok(s) = self.store.load_session(String::from(id)).await {
            if let Some(s) = s {
                if !s.is_expired() {
                    return Some(s);
                }
                self.store.destroy_session(s).await;
            }
        }
        None
    }

    fn ttl(&self, reg: bool) -> u64 {
        let day = 60 * 60 * 24;
        if reg {
            return day * 365;
        }
        day * 3
    }
}
