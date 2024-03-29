use crate::lichess::MyKey;

use self::{mongo::Mongo, redis::RedisCli};

pub mod mongo;
pub mod queries;
pub mod redis;
pub mod serde_helpers;

/// Struct containing all databases.
pub struct Database {
    pub redis: RedisCli,
    pub mongo: Mongo,
    pub key: MyKey,
}

impl Database {
    /// Create databases.
    pub async fn new() -> Self {
        let redis = RedisCli::default().await;
        let mongo = Mongo::new().await;
        let key = MyKey::default();
        Self { redis, mongo, key }
    }
}
