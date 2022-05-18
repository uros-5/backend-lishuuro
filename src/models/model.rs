use bson::{doc, oid::ObjectId};
use glicko2::{GameResult, GlickoRating};
use json_value_merge::Merge;
use mongodb::Collection;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use shuuro::Color;
use std::hash::{Hash, Hasher};
use std::{collections::HashMap, time::Duration as StdD};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::lichess::login::random_username;

pub const VARIANTS: [&str; 1] = ["shuuro12"];
pub const DURATION_RANGE: [i64; 28] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30, 35, 40, 45, 60,
    75, 90,
];

// APP MODELS
pub struct AppState {
    pub users: Collection<User>,
    pub news: Collection<NewsItem>,
    pub games: Collection<ShuuroGame>,
    pub counter: u8,
    pub login_state: String,
}

impl AppState {
    pub fn new(
        users: Collection<User>,
        news: Collection<NewsItem>,
        games: Collection<ShuuroGame>,
        login_state: String,
    ) -> Self {
        AppState {
            users,
            news,
            games,
            login_state: login_state,
            counter: 0,
        }
    }
}

// MONGODB MODELS

/// Representing user in database.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub _id: String,
    pub username: String,
    pub active: bool,
    pub currently_playing: bool,
    pub created_at: String,
    pub last_games: Vec<PlayerMatch>,
    pub rating: f64,
    pub deviation: f64,
}

impl User {
    pub fn new(username: &String) -> Self {
        User {
            _id: String::from(username),
            username: String::from(username),
            active: true,
            currently_playing: false,
            created_at: String::from(""),
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
            let username = random_username();
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
    pub white: String,
    pub black: String,
    pub side_to_move: String,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub white_clock: Duration,
    #[serde(serialize_with = "duration_i32")]
    #[serde(deserialize_with = "i32_duration")]
    pub black_clock: Duration,
    #[serde(serialize_with = "date_str")]
    #[serde(deserialize_with = "str_date")]
    pub last_clock: OffsetDateTime,
    pub current_stage: u8,
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
    pub ratings: HashMap<String, [f64; 2]>,
}

impl Default for ShuuroGame {
    fn default() -> Self {
        Self {
            _id: ObjectId::new(),
            game_id: String::from(""),
            min: Duration::default(),
            incr: Duration::default(),
            white: String::from(""),
            black: String::from(""),
            side_to_move: String::from(""),
            white_clock: Duration::default(),
            black_clock: Duration::default(),
            last_clock: OffsetDateTime::now_utc(),
            current_stage: 0,
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
            ratings: HashMap::new(),
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

    /// Return Color::NoColor if user does not exist.
    pub fn user_color(&self, username: &String) -> Color {
        if username == &self.white {
            Color::White
        } else if username == &self.black {
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

impl From<&LobbyGame> for ShuuroGame {
    fn from(game: &LobbyGame) -> Self {
        ShuuroGame::new(game.time, game.incr)
    }
}

/// NewsItem for main page.
#[derive(Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub _id: ObjectId,
    pub title: String,
    pub user: String,
    pub date: String,
    pub category: String,
    pub text: String,
    pub headline: String,
}

// WEBSOCKETS

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatItem {
    pub id: String,
    pub user: String,
    pub time: String,
    pub message: String,
}

impl ChatItem {
    pub fn date(&self) -> String {
        String::from(&self.time)
    }

    /// Formats date in format HH:MM
    pub fn update(&mut self, user: &String) {
        let now = OffsetDateTime::now_utc();
        self.user = String::from(user);
        self.time = format!("{}:{}", now.hour(), now.minute());
    }

    /// Formats ChatItem for json response.
    pub fn response(&mut self) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": "live_chat_message" });
        first.merge(second);
        first
    }
}

#[derive(Clone)]
pub struct ChatRooms {
    rooms: HashMap<String, Vec<ChatItem>>,
}

impl ChatRooms {
    /// Creates ChatRooms, with already inserted "home"
    pub fn new() -> Self {
        let mut rooms = HashMap::default();
        rooms.insert(String::from("home"), vec![]);
        let room = ChatRooms { rooms };
        room
    }

    /// Check if user can add new message.
    pub fn can_add(chat: &Vec<ChatItem>, player: &ActivePlayer) -> bool {
        let count = chat.iter().fold(0, |mut acc, x| {
            if &x.user == &player.username() {
                acc += 1;
            }
            acc
        });

        if !&player.reg() {
            return false;
        } else if count == 5 {
            return false;
        }

        true
    }

    /// Check if message length is in valid range
    pub fn message_length(m: &ChatItem) -> bool {
        if m.message.len() > 0 && m.message.len() < 50 {
            return true;
        }
        false
    }

    /// Add new message, before checking out
    pub fn add_msg(
        &mut self,
        id: &String,
        m: &mut ChatItem,
        player: &ActivePlayer,
    ) -> Option<Value> {
        if let Some(chat) = self.rooms.get_mut(id) {
            if ChatRooms::message_length(&m) {
                if ChatRooms::can_add(&chat, &player) {
                    m.update(&player.username());
                    let res = m.response();
                    chat.push(m.clone());
                    return Some(res);
                    //return self.send_message_to_all(res);
                }
            }
        }
        None
    }

    /// Returns entire chat room
    pub fn chat(&self, id: &String) -> Option<&Vec<ChatItem>> {
        if let Some(chat) = self.rooms.get(id) {
            return Some(chat);
        }
        None
    }

    /// Insert new room
    pub fn add_room(&mut self, id: String) {
        self.rooms.insert(id, vec![]);
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActivePlayer {
    reg: bool,
    username: String,
}

impl ActivePlayer {
    pub fn new(reg: &bool, username: &String) -> Self {
        ActivePlayer {
            reg: *reg,
            username: String::from(username),
        }
    }
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn reg(&self) -> bool {
        self.reg.clone()
    }
}

impl PartialEq for ActivePlayer {
    fn eq(&self, other: &Self) -> bool {
        self.username == other.username
    }

    fn ne(&self, other: &Self) -> bool {
        self.username != other.username
    }
}

impl Eq for ActivePlayer {}

impl Hash for ActivePlayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.username.hash(state);
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
    /// Return true if game has valid time.
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

    /// Formats game for json response.
    pub fn response(&mut self, t: &String) -> Value {
        let mut first = serde_json::json!(&mut self.clone());
        let second = json!({ "t": t });
        first.merge(second);

        first
    }

    /// Return id for game
    pub fn username(&self) -> String {
        String::from(&self.username)
    }

    /// Returns player colors
    pub fn colors(&mut self, other: &String) -> [String; 2] {
        let mut c_s: [String; 2] = [String::from(""), String::from("")];
        let mut color = String::from(self.color());
        let other = String::from(other);
        let me = self.username();
        if color == "random" {
            color = self.random_color();
        }
        if color == "white" {
            c_s = [me, other];
        }
        // this is black
        else {
            c_s = [other, me];
        }
        c_s
    }

    ///
    pub fn color(&self) -> &String {
        &self.color
    }

    fn random_color(&self) -> String {
        if rand::random() {
            String::from("white")
        } else {
            String::from("black")
        }
    }
}

impl PartialEq for LobbyGame {
    fn eq(&self, other: &LobbyGame) -> bool {
        self.username == other.username
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TvGame {
    pub t: String,
    pub game_id: String,
    pub w: String,
    pub b: String,
    pub sfen: String,
}

impl TvGame {
    pub fn new(t: &str, game_id: &str, w: &str, b: &str, fen: &str) -> Self {
        Self {
            t: String::from(t),
            game_id: String::from(game_id),
            w: String::from(w),
            b: String::from(b),
            sfen: String::from(fen),
        }
    }
}

#[derive(Clone)]
pub struct LobbyGames {
    all: Vec<LobbyGame>,
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
        }
    }
}

impl LobbyGames {
    pub fn can_add(&self, game: &LobbyGame) -> bool {
        for i in &self.all {
            if i == game {
                return false;
            }
        }

        return true;
    }
    pub fn add(&mut self, game: LobbyGame) {
        self.all.push(game);
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
    stage: u8,
}

impl TimeControl {
    pub fn new(inc: i64, duration: i64) -> Self {
        TimeControl {
            last_click: OffsetDateTime::now_utc(),
            inc: Duration::new(inc, 0),
            black_player: Duration::new(duration, 0),
            white_player: Duration::new(duration, 0),
            stage: 0,
        }
    }

    pub fn update_stage(&mut self, stage: u8) {
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
        if self.stage != 0 {
            self.last_click = OffsetDateTime::now_utc();
        }
        self.time_ok(&c)
    }

    pub fn time_ok(&self, c: &String) -> bool {
        if self.stage == 0 {
            if c == "" {
                return (self.white_player - self.elapsed()).whole_milliseconds() > 0;
            }
        }
        if c == "w" {
            return (self.white_player - self.elapsed()).whole_milliseconds() > 0;
        } else if c == "b" {
            return (self.black_player - self.elapsed()).whole_milliseconds() > 0;
        }
        false
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

impl From<&ShuuroGame> for TimeControl {
    fn from(game: &ShuuroGame) -> Self {
        let inc = game.incr.whole_seconds();
        let duration = game.min.whole_seconds();
        let white = game.white_clock;
        let black = game.black_clock;
        let last_click = game.last_clock;
        let mut tc = Self::new(inc, duration);
        tc.white_player = white;
        tc.black_player = black;
        tc.last_click = last_click;
        tc
    }
}

fn date_str<S>(x: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date = x.to_string();
    let date = date.split(" +").next().unwrap().clone();
    s.serialize_str(&date)
}

fn str_date<'de, D>(data: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(data)?;
    let s = String::from(s);
    let s = s.split(".").next().unwrap();
    let format = "%F %T";
    match PrimitiveDateTime::parse(&s, format) {
        Ok(d) => {
            //return Ok(i);
            return Ok(d.assume_utc());
        }
        Err(_e) => {
            return Ok(OffsetDateTime::now_utc());
        }
    }
}

fn duration_i32<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = x.whole_milliseconds() as u64;
    s.serialize_u64(duration)
}

fn i32_duration<'de, D>(data: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: u64 = Deserialize::deserialize(data)?;
    let d2 = StdD::from_millis(s);
    Ok(Duration::new(d2.as_secs() as i64, d2.as_nanos() as i32))
}
