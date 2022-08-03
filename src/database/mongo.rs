use async_session::chrono::Duration;
use bson::DateTime;
use mongodb::{options::ClientOptions, Client, Collection};
use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration as StdD;


// MONGODB MODELS

#[derive(Clone)]
pub struct Mongo {
    pub players: Collection<Player>,
    pub articles: Collection<Article>,
    pub games: Collection<ShuuroGame>,
}

impl Mongo {
    pub async fn new() -> Self {
        let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017")
            .await
            .expect("No client available");
        client_options.app_name = Some("lishuuro".to_string());
        let client = Client::with_options(client_options).expect("client not found");
        let db = client.database("lishuuro");
        let players = db.collection::<Player>("users");
        let games = db.collection::<ShuuroGame>("shuuroGames");
        let articles = db.collection::<Article>("news");
        Mongo {
            players,
            games,
            articles,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Representing one player
pub struct Player {
    pub _id: String,
    pub reg: bool,
    pub created_at: DateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// News for main page.
pub struct Article {
    pub _id: String,
    pub title: String,
    pub user: String,
    pub date: String,
    pub category: String,
    pub text: String,
    pub headline: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShuuroGame {
    pub _id: String,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub min: Duration,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub incr: Duration,
    pub players: [String; 2],
    pub side_to_move: u8,
    #[serde(serialize_with = "duration_i32_array")]
    #[serde(deserialize_with = "array_i32_duration")]
    pub clocks: [Duration; 2],
    pub last_clock: DateTime,
    pub current_stage: u8,
    pub result: String,
    pub status: i32,
    pub credits: [u16; 2],
    pub hands: [String; 2],
    pub history: [(String, u8); 3],
    pub sfen: String,
}

// Serde helpers

/// Serializing from Duration to String
fn duration_i32<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = x.num_milliseconds() as u64;
    s.serialize_u64(duration)
}

/// Serializing from String to Duration
fn i32_duration<'de, D>(data: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: u64 = Deserialize::deserialize(data)?;
    let d2 = StdD::from_millis(s);
    if let Ok(d2) = Duration::from_std(d2) {
        return Ok(d2);
    }
    Ok(Duration::minutes(1))
}

/// Serializing from [Duration; 2] to String
fn duration_i32_array<S>(x: &[Duration; 2], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut tup = s.serialize_tuple(2)?;
    for (_i, duration) in x.iter().enumerate() {
        let value = duration.num_milliseconds() as u64;
        tup.serialize_element(&value).unwrap();
    }
    return Ok(tup.end().ok().unwrap());
}

/// Deserializing from String to [Duration; 2]
fn array_i32_duration<'de, D>(data: D) -> Result<[Duration; 2], D::Error>
where
    D: Deserializer<'de>,
{
    let s: [u64; 2] = Deserialize::deserialize(data)?;
    let mut durations = [Duration::seconds(1); 2];
    for (i, u) in s.iter().enumerate() {
        let d2 = StdD::from_millis(*u);
        if let Ok(d) = Duration::from_std(d2) {
            durations[i] = d;
        }
    }
    Ok(durations)
}