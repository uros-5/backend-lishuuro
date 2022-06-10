mod sd;

use json_value_merge::Merge;
use mongodb::bson::oid::ObjectId;
use mongodb::{options::ClientOptions, Client};
use mongodb::{Collection, Database};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use serde_with::serde_as;
use shuuro::Color;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use time::{OffsetDateTime, PrimitiveDateTime};
use sd::*;

// MONGODB MODELS

/// Representing user in database.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub _id: String,
    pub username: String,
    pub last_games: Vec<PlayerMatch>,
    pub rating: f64,
    pub deviation: f64,
}

impl User {
    pub fn new(username: &String) -> Self {
        User {
            _id: String::from(username),
            username: String::from(username),
            last_games: vec![],
            rating: 1500.00,
            deviation: 300.00,
        }
    }

    pub fn merge(&mut self, reg: bool) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "reg": reg });
        first.merge(second);
        first
    }

    pub async fn db_username(db: &Collection<User>) -> String {
        let mut _duplicate = true;
        while _duplicate {
            let username = String::from(""); //random_username();
            let anon = User::new(&username);
            let res = db.insert_one(&anon, None).await;
            if let Ok(_) = res {
                return String::from(&username);
            }
        }
        String::from("")
    }
}

/// Model used for user recent results.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerMatch {
    pub r: f64,
    pub d: f64,
    pub o: String,
}

impl PlayerMatch {
    pub fn new(rating: &[f64; 2], o: &str) -> Self {
        Self {
            r: rating[0],
            d: rating[1],
            o: String::from(o),
        }
    }
}

/*
impl From<&PlayerMatch> for GameResult {
    fn from(m: &PlayerMatch) -> Self {
        let r = GlickoRating {
            value: m.r,
            deviation: m.d,
        };
        match m.o.as_str() {
            "w" => GameResult::win(r),
            "l" => GameResult::loss(r),
            _ => GameResult::draw(r),
        }
    }
}
*/

/// ShuuroGame representation in database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuuroGame {
    pub _id: ObjectId,
    pub game_id: String,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub min: Duration,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub incr: Duration,
    pub players: [String; 2],
    pub side_to_move: String,
    #[serde(serialize_with = "clocks_to")]
    #[serde(deserialize_with = "to_clocks")]
    pub clocks: (Duration, Duration),
    pub credits: [u16; 2],
    pub hands: [String; 2],
    #[serde(serialize_with = "date_str")]
    #[serde(deserialize_with = "str_date")]
    pub last_clock: OffsetDateTime,
    pub current_stage: u8,
    pub result: String,
    pub status: i32,
    pub shop_history: Vec<(String, u8)>,
    pub deploy_history: Vec<(String, u16)>,
    pub fight_history: Vec<(String, u16)>,
    pub sfen: String,
    pub ratings: HashMap<String, [f64; 2]>,
}

impl Default for ShuuroGame {
    fn default() -> Self {
        Self {
            _id: ObjectId::new(),
            game_id: String::from(""),
            min: Duration::default(),
            incr: Duration::default(),
            players: [String::from(""), String::from("")],
            clocks: (Duration::default(), Duration::default()),
            side_to_move: String::from(""),
            last_clock: OffsetDateTime::now_utc(),
            current_stage: 0,
            result: String::from(""),
            status: -2,
            shop_history: Vec::new(),
            deploy_history: Vec::new(),
            fight_history: Vec::new(),
            credits: [800, 800],
            hands: [String::from(""), String::from("")],
            sfen: String::from(""),
            ratings: HashMap::new(),
        }
    }
}

impl ShuuroGame {
    fn new(time: u64, incr: u64) -> Self {
        let mut game = ShuuroGame::default();
        let min_seconds = time * 60;
        game.min = Duration::new(min_seconds, 0);
        game.incr = Duration::new(incr, 0);
        game.clocks.0 = Duration::new(min_seconds, 0);
        game.clocks.1 = Duration::new(min_seconds, 0);
        game
    }

    /// Return Color::NoColor if user does not exist.
    pub fn user_color(&self, username: &String) -> Color {
        if username == &self.players[0] {
            Color::White
        } else if username == &self.players[1] {
            Color::Black
        } else {
            Color::NoColor
        }
    }

    /// Get ratings from db and updates here.
    pub fn update_ratings(&mut self, ratings: HashMap<String, [f64; 2]>) {
        self.ratings = ratings;
    }
}

/*
impl From<&LobbyGame> for ShuuroGame {
    fn from(game: &LobbyGame) -> Self {
        ShuuroGame::new(game.time, game.incr)
    }
}
*/

/// NewsItem for main page.
#[derive(Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub _id: ObjectId,
    pub title: String,
    pub user: String,
    pub date: String,
    pub category: String,
    pub text: String,
    pub headline: String,
}

pub struct DbCols {
    pub users: Collection<User>,
    pub games: Collection<ShuuroGame>,
    pub news: Collection<NewsArticle>
}

impl DbCols {
    pub async fn new() -> Self {
            let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017")
            .await
            .expect("No client available");
        client_options.app_name = Some("lishuuro2".to_string());
        let client = Client::with_options(client_options).expect("client not found");
        let db = client.database("lishuuro2");
        let users = db.collection::<User>("users");
        let games = db.collection::<ShuuroGame>("shuuroGames");
        let news = db.collection::<NewsArticle>("news");
        Self {
            users,games,news
        }
    }
}
