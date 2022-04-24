use json_value_merge::Merge;
use mongodb::Collection;
use rand::prelude::*;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use shuuro::{Color, Position, Shop};
use time::{Duration, OffsetDateTime};

pub const VARIANTS: [&str; 1] = ["shuuro12"];
pub const DURATION_RANGE: [i64; 28] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30, 35, 40, 45, 60,
    75, 90,
];

// APP MODELS
pub struct AppState {
    pub users: Collection<User>,
    pub news: Collection<NewsItem>,
    pub counter: u8,
}

impl AppState {
    pub fn new(users: Collection<User>, news: Collection<NewsItem>) -> Self {
        AppState {
            users,
            news,
            counter: 0,
        }
    }
    pub fn update_counter(&mut self) {
        self.counter += 1
    }
}

// MONGODB MODELS
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub _id: String,
    pub username: String,
    pub active: bool,
    pub currently_playing: bool,
    pub created_at: String,
}

impl User {
    pub fn new(username: String) -> Self {
        User {
            _id: username.clone(),
            username,
            active: true,
            currently_playing: false,
            created_at: String::from(""),
        }
    }

    pub fn merge(&mut self, reg: bool) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "reg": reg });
        first.merge(second);
        first
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuuroGame {
    #[serde(serialize_with = "duration_i32")]
    pub min: Duration,
    #[serde(serialize_with = "duration_i32")]
    pub incr: Duration,
    pub white: String,
    pub black: String,
    pub side_to_move: String,
    #[serde(serialize_with = "duration_i32")]
    pub white_clock: Duration,
    #[serde(serialize_with = "duration_i32")]
    pub black_clock: Duration,
    #[serde(serialize_with = "date_str")]
    pub last_clock: OffsetDateTime,
    pub current_stage: String,
    pub result: String,
    pub status: i32,
    pub shop_history: Vec<(String, u8)>,
    pub deploy_history: Vec<(String, u16)>,
    pub fight_history: Vec<(String, u16)>,
    pub white_credit: u16,
    pub black_credit: u16,
    pub white_hand: String,
    pub black_hand: String,
    pub sfen: String,
}

impl Default for ShuuroGame {
    fn default() -> Self {
        Self {
            min: Duration::default(),
            incr: Duration::default(),
            white: String::from(""),
            black: String::from(""),
            side_to_move: String::from(""),
            white_clock: Duration::default(),
            black_clock: Duration::default(),
            last_clock: OffsetDateTime::now_utc(),
            current_stage: String::from("shop"),
            result: String::from(""),
            status: -2,
            shop_history: Vec::new(),
            deploy_history: Vec::new(),
            fight_history: Vec::new(),
            white_credit: 800,
            black_credit: 800,
            white_hand: String::from(""),
            black_hand: String::from(""),
            sfen: String::from(""),
        }
    }
}

impl ShuuroGame {
    fn new(time: i64, incr: i64) -> Self {
        let mut game = ShuuroGame::default();
        let min_seconds = time * 60;
        game.min = Duration::new(min_seconds, 0);
        game.incr = Duration::new(incr, 0);
        game.white_clock = Duration::new(min_seconds, 0);
        game.black_clock = Duration::new(min_seconds, 0);
        game
    }
    pub fn user_color(&self, username: &String) -> Color {
        if username == &self.white {
            Color::White
        } else if username == &self.black {
            Color::Black
        } else {
            Color::NoColor
        }
    }
}

impl From<&LobbyGame> for ShuuroGame {
    fn from(game: &LobbyGame) -> Self {
        ShuuroGame::new(game.time, game.incr)
    }
}

// WebSockets
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatItem {
    pub user: String,
    pub time: String,
    pub message: String,
}

impl ChatItem {
    pub fn new(user: &String, message: &String) -> Self {
        let now = OffsetDateTime::now_utc();
        ChatItem {
            user: user.clone(),
            message: message.clone(),
            time: format!("{}:{}", now.hour(), now.minute()),
        }
    }
    pub fn date(&self) -> String {
        self.time.clone()
    }
    pub fn update(&mut self, user: &String) {
        let now = OffsetDateTime::now_utc();
        self.user = user.clone();
        self.time = format!("{}:{}", now.hour(), now.minute());
    }

    pub fn response(&mut self) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": "home_chat_message" });
        first.merge(second);
        first
    }
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub struct ActivePlayer {
    reg: bool,
    username: String,
}

impl ActivePlayer {
    pub fn new(reg: &bool, username: &String) -> Self {
        ActivePlayer {
            reg: *reg,
            username: username.clone(),
        }
    }
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn reg(&self) -> bool {
        self.reg.clone()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LobbyGame {
    username: String,
    variant: String,
    time: i64,
    incr: i64,
    color: String,
}

impl LobbyGame {
    pub fn is_valid(&self) -> bool {
        if VARIANTS.contains(&&self.variant[..]) {
            if DURATION_RANGE.contains(&self.time) {
                if DURATION_RANGE.contains(&self.incr) {
                    return true;
                } else if &self.incr == &0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn response(&mut self, t: &String) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": t });
        first.merge(second);

        first
    }

    pub fn username(&self) -> String {
        self.username.clone()
    }

    pub fn colors(&mut self, accepting_player: &String) -> [String; 2] {
        let mut c_s: [String; 2] = [String::from(""), String::from("")];
        let mut temp_color = self.color.clone();
        if temp_color == String::from("random") {
            if rand::random() {
                temp_color = String::from("white");
            } else {
                temp_color = String::from("black");
            }
        }
        if temp_color == String::from("white") {
            c_s = [self.username(), accepting_player.clone()];
        }
        // this is black
        else {
            c_s = [accepting_player.clone(), self.username()];
        }
        c_s
    }

    pub fn color(&self) -> &String {
        &self.color
    }
}

impl PartialEq for LobbyGame {
    fn eq(&self, other: &LobbyGame) -> bool {
        self.username == other.username
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameRequest {
    pub t: String,
    pub color: String,
    pub game_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameMove {
    pub t: String,
    pub game_id: String,
    pub game_move: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameGetHand {
    pub t: String,
    pub game_id: String,
    pub color: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameGetConfirmed {
    pub t: String,
    pub game_id: String,
}

#[derive(Clone)]
pub struct LobbyGames {
    all: Vec<LobbyGame>,
    duration_range: Vec<u8>,
}

impl Default for LobbyGames {
    fn default() -> Self {
        let mut all = vec![];
        for i in 1..20 {
            all.push(i)
        }
        for i in (20..45).step_by(5) {
            all.push(i)
        }
        for i in (45..100).step_by(15) {
            all.push(i)
        }
        LobbyGames {
            all: Vec::<LobbyGame>::new(),
            duration_range: all,
        }
    }
}

impl LobbyGames {
    pub fn can_add(&self, game: &LobbyGame) -> bool {
        for i in &self.all {
            if *i == *game {
                return false;
            }
        }

        return true;
    }
    pub fn add(&mut self, game: LobbyGame) {
        self.all.push(game);
    }

    pub fn all(&self) -> &Vec<LobbyGame> {
        &self.all
    }

    pub fn delete(&mut self, game: LobbyGame) -> i32 {
        let index = self.all.iter().position(|x| *x == game);
        match index {
            Some(i) => {
                self.all.remove(i);
                return i as i32;
            }
            None => -10,
        }
    }

    pub fn delete_by_user(&mut self, user: &ActivePlayer) -> bool {
        let index = self
            .all
            .iter()
            .position(|x| *x.username() == user.username());
        match index {
            Some(i) => {
                self.all.remove(i);
                return true;
            }
            None => false,
        }
    }

    pub fn response(&self) -> Value {
        json!({ "t": "home_lobby_full", "lobbyGames": &self.all})
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TimeControl {
    #[serde(serialize_with = "date_str")]
    last_click: OffsetDateTime,
    inc: Duration,
    #[serde(serialize_with = "duration_i32")]
    black_player: Duration,
    #[serde(serialize_with = "duration_i32")]
    white_player: Duration,
    stage: String,
}

impl TimeControl {
    pub fn new(inc: i64, duration: i64) -> Self {
        TimeControl {
            last_click: OffsetDateTime::now_utc(),
            inc: Duration::new(inc, 0),
            black_player: Duration::new(duration, 0),
            white_player: Duration::new(duration, 0),
            stage: String::from("shop"),
        }
    }

    pub fn update_stage(&mut self, stage: String) {
        self.stage = stage;
        self.last_click = OffsetDateTime::now_utc();
    }

    pub fn click(&mut self, color: Color) -> bool {
        let elapsed = self.elapsed();
        let c = color.to_string();
        if c == "w" {
            self.white_player -= elapsed;
            self.white_player += self.inc;
        } else if c == "b" {
            self.black_player -= elapsed;
            self.black_player += self.inc;
        }
        if self.stage != "shop" {
            self.last_click = OffsetDateTime::now_utc();
        }
        self.time_ok(&c)
    }

    pub fn time_ok(&self, c: &String) -> bool {
        if self.stage == String::from("shop") {
            return (self.white_player - self.elapsed()).whole_milliseconds() > 0;
        }
        if c == "w" {
            return self.white_player.whole_milliseconds() > 0;
        } else if c == "b" {
            return self.black_player.whole_milliseconds() > 0;
        }
        false
    }

    pub fn set_clock(&mut self, c: char, d: Duration) {
        if c == 'w' {
            self.white_player = d;
        } else if c == 'b' {
            self.black_player = d;
        }
    }

    pub fn get_clock(&self, c: char) -> Duration {
        if c == 'w' {
            self.white_player
        } else if c == 'b' {
            self.black_player
        } else {
            self.white_player
        }
    }

    pub fn get_last_click(&self) -> OffsetDateTime {
        self.last_click
    }

    fn elapsed(&self) -> Duration {
        OffsetDateTime::now_utc() - self.last_click
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub user: String,
    #[serde(serialize_with = "date_str")]
    pub date: OffsetDateTime,
    pub category: String,
    pub text: String,
    pub headline: String,
}

fn date_str<S>(x: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date = x.to_string();
    let date = date.split(" +").next().unwrap().clone();
    s.serialize_str(&date)
}

fn duration_i32<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = x.whole_milliseconds() as i64;
    s.serialize_i64(duration)
}
