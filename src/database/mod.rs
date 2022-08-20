use async_session::chrono::Duration;
use bson::DateTime;
use mongodb::{options::ClientOptions, Client, Collection};
use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration as StdD;

use crate::lichess::MyKey;

use self::{mongo::Mongo, redis::RedisCli};

pub mod mongo;
pub mod queries;
pub mod redis;

pub struct Database {
    pub redis: RedisCli,
    pub mongo: Mongo,
    pub key: MyKey,
}

impl Database {
    pub async fn new() -> Self {
        let redis = RedisCli::default().await;
        let mongo = Mongo::new().await;
        let key = MyKey::default();
        Self { redis, mongo, key }
    }
}
