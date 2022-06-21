use serde::{Deserialize, Deserializer, Serialize, Serializer};
use mongodb::{Collection, Database, options::ClientOptions, Client};
use async_session::chrono::{DateTime, Utc, Duration};
use crate::models::sd::*;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShuuroGame {
    pub _id: String,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub min: Duration,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub incr: Duration,
    pub players: [String; 2],
    #[serde(serialize_with = "clocks_i32")]
    #[serde(deserialize_with = "i32_clocks")]
    pub clocks: [Duration; 2],
    pub credits: [u8; 2],
    pub hands: [String; 2],
    pub stm: String,
    pub last_clock: DateTime<Utc>, 
    pub current_stage: u8,
    pub result: String,
    pub status: u8,
    pub history: [(String, u16); 3],
    pub sfen: String 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub _id: String,
    pub username: String,
    pub reg: bool
}

impl User {
    pub fn new(username: String) -> Self {
        Self {username: String::from(&username),_id: username, reg: false}
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewsArticle {
    pub _id: String,
    pub title: String,
    pub user: String,
    pub date: String,
    pub category: String,
    pub text: String,
    pub headline: String
}

pub struct Db {
    pub shuuro_games: Collection<ShuuroGame>,
    pub users: Collection<User>,
    pub news: Collection<NewsArticle>
}

impl Db {
    async fn new() -> Self {
        let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017")
        .await
        .expect("No client available");
        client_options.app_name = Some("lishuuro".to_string());
        let client = Client::with_options(client_options).expect("client not found");
        let db = client.database("lishuuro");
        let users = db.collection::<User>("users");
        let shuuro_games = db.collection::<ShuuroGame>("shuuroGames");
        let news = db.collection::<NewsArticle>("news");
        Self { users, shuuro_games, news }
    }
}
