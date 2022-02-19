use json_value_merge::Merge;
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};

pub const VARIANTS: [&str; 1] = ["shuuro12"];
pub const DURATION_RANGE: [i64; 28] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 25, 30, 35, 40, 45, 60,
    75, 90,
];

// APP MODELS
pub struct AppState {
    pub users: Collection<User>,
    pub counter: u8,
}

impl AppState {
    pub fn new(users: Collection<User>) -> Self {
        AppState { users, counter: 0 }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShuuroStage {
    pub stm: String,
    pub white_hand: String,
    pub black_hand: String,
    pub fen: Vec<String>,
}

impl Default for ShuuroStage {
    fn default() -> Self {
        ShuuroStage {
            stm: String::default(),
            white_hand: String::default(),
            black_hand: String::default(),
            fen: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShuuroGame {
    pub time: Duration,
    pub incr: Duration,
    pub white: String,
    pub black: String,
    pub stm: String,
    pub white_clock: Duration,
    pub black_clock: Duration,
    pub last_clock: OffsetDateTime,
    pub current_stage: String,
    pub result: String,
    pub shop: ShuuroStage,
    pub put: ShuuroStage,
    pub play: ShuuroStage,
}

impl Default for ShuuroGame {
    fn default() -> Self {
        Self {
            time: Duration::default(),
            incr: Duration::default(),
            white: String::from(""),
            black: String::from(""),
            stm: String::from("white"),
            white_clock: Duration::default(),
            black_clock: Duration::default(),
            last_clock: OffsetDateTime::now_utc(),
            current_stage: String::from("shop"),
            result: String::from(""),
            shop: ShuuroStage::default(),
            put: ShuuroStage::default(),
            play: ShuuroStage::default(),
        }
    }
}

impl ShuuroGame {
    fn new(time: i64, incr: i64) -> Self {
        let mut game = ShuuroGame::default();
        game.time = Duration::new(time * 60, 0);
        game.incr = Duration::new(incr, 0);
        game
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
}

impl PartialEq for LobbyGame {
    fn eq(&self, other: &LobbyGame) -> bool {
        self.username == other.username
            && self.variant == other.variant
            && self.time == other.time
            && self.incr == other.incr
    }
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

    pub fn delete_by_user(&mut self, user: &ActivePlayer) {
        self.all.retain(|item| &item.username() != &user.username());
        /*for game in self.all.iter() {

        }*/
    }

    pub fn response(&self) -> Value {
        json!({ "t": "home_lobby_full", "lobbyGames": &self.all})
    }
}

#[derive(Serialize, Deserialize)]
pub struct TimeControl {
    last_click: OffsetDateTime,
    inc: u8,
    black_player: Duration,
    white_player: Duration,
}

impl TimeControl {
    fn new(inc: u8, duration: i64) -> Self {
        TimeControl {
            last_click: OffsetDateTime::now_utc(),
            inc,
            black_player: Duration::new(duration, 0),
            white_player: Duration::new(duration, 0),
        }
    }

    fn click(&mut self, c: char) -> bool {
        let now = OffsetDateTime::now_utc();
        let elapsed = now - self.last_click;
        if c == 'w' {
            self.white_player -= elapsed;
        } else if c == 'b' {
            self.black_player -= elapsed;
        }
        self.last_click = now;
        self.time_ok(c)
    }

    fn time_ok(&self, c: char) -> bool {
        if c == 'w' {
            return self.white_player.whole_seconds() <= 0
                && self.white_player.whole_nanoseconds() <= 0;
        } else if c == 'b' {
            return self.black_player.whole_seconds() <= 0
                && self.white_player.whole_nanoseconds() <= 0;
        }
        false
    }
}
