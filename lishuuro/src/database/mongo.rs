use async_session::chrono::Duration;
use bson::DateTime;
use mongodb::{options::ClientOptions, Client, Collection};
use serde::{Deserialize, Serialize};
use shuuro::SubVariant;

use crate::websockets::{time_control::TimeControl, GameRequest};

use super::serde_helpers::{
    array_i32_duration, duration_i32, duration_i32_array, i32_duration,
};
use crate::database::serde_helpers::{
    deserialize_subvariant, serialize_subvariant,
};

// MONGODB MODELS

#[derive(Clone)]
pub struct Mongo {
    pub players: Collection<Player>,
    pub articles: Collection<Article>,
    pub games: Collection<ShuuroGame>,
}

impl Mongo {
    /// Create mongodb connection for all collections.
    pub async fn new() -> Self {
        let mut client_options =
            ClientOptions::parse("mongodb://127.0.0.1:27017")
                .await
                .expect("No client available");
        client_options.app_name = Some("lishuuro".to_string());
        let client =
            Client::with_options(client_options).expect("client not found");
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

pub type History = (Vec<String>, Vec<String>, Vec<String>);

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
    pub variant: String,
    pub credits: [u16; 2],
    pub hands: [String; 2],
    pub sfen: String,
    pub history: History,
    pub tc: TimeControl,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub draws: [bool; 2],
    #[serde(serialize_with = "serialize_subvariant")]
    #[serde(deserialize_with = "deserialize_subvariant")]
    pub sub_variant: Option<SubVariant>,
}

impl From<(&GameRequest, &[String; 2], &str)> for ShuuroGame {
    fn from(f: (&GameRequest, &[String; 2], &str)) -> Self {
        let clock = Duration::seconds(60 * f.0.time + f.0.incr);
        Self {
            _id: String::from(f.2),
            min: Duration::seconds(f.0.time * 60),
            incr: Duration::seconds(f.0.incr),
            players: f.1.clone(),
            side_to_move: 0,
            clocks: [clock, clock],
            last_clock: DateTime::now(),
            current_stage: 0,
            result: String::from(""),
            status: -2,
            variant: String::from(&f.0.variant),
            credits: [800, 800],
            hands: [String::from(""), String::from("")],
            sfen: String::from(""),
            history: (vec![], vec![], vec![]),
            tc: TimeControl::new(f.0.time, f.0.incr),
            draws: [false, false],
            sub_variant: f.0.sub_variant,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileGame {
    pub _id: String,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub min: Duration,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub incr: Duration,
    pub players: [String; 2],
    pub side_to_move: u8,
    pub last_clock: DateTime,
    pub current_stage: u8,
    pub result: String,
    pub status: i32,
    pub variant: String,
    pub sfen: String,
    pub tc: TimeControl,
    #[serde(serialize_with = "serialize_subvariant")]
    #[serde(deserialize_with = "deserialize_subvariant")]
    pub sub_variant: Option<SubVariant>,
}
