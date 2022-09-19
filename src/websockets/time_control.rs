use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::database::mongo::ShuuroGame;
use crate::database::serde_helpers::{array_i32_duration, duration_i32_array};

/// TimeControl for ShuuroGame.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeControl {
    pub last_click: DateTime<FixedOffset>,
    #[serde(serialize_with = "duration_i32_array")]
    #[serde(deserialize_with = "array_i32_duration")]
    pub clocks: [Duration; 2],
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub stage: u8,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub incr: i64,
}

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::new(10, 5)
    }
}

impl From<&ShuuroGame> for TimeControl {
    fn from(s: &ShuuroGame) -> Self {
        let last_click = DateTime::parse_from_str(&s.last_clock.to_string(), "%+").unwrap();
        Self {
            last_click: last_click,
            clocks: s.clocks.clone(),
            stage: s.current_stage,
            incr: s.incr.num_seconds(),
        }
    }
}

impl TimeControl {
    /// Create new time control.
    pub fn new(time: i64, incr: i64) -> Self {
        let duration = Duration::seconds(time * 60 + incr);
        let last_click = Utc::now().into();

        Self {
            clocks: [duration, duration.clone()],
            stage: 0,
            incr,
            last_click,
        }
    }

    /// Update to current stage.
    pub fn update_stage(&mut self, stage: u8) {
        self.stage = stage;
        self.last_click = Utc::now().into();
    }

    /// Click on clock. For shop both can click.
    pub fn click(&mut self, color: usize) -> Option<[u64; 2]> {
        if let Some(duration) = self.current_duration(color) {
            self.update_last_click(color, duration);
            let ms = [
                self.clocks[0].num_milliseconds() as u64,
                self.clocks[1].num_milliseconds() as u64,
            ];
            return Some(ms);
        }
        None
    }

    /// Get current duration for selected color.
    pub fn current_duration(&self, color: usize) -> Option<Duration> {
        let elapsed = self.elapsed();
        if let Some(duration) = self.clocks[color].checked_sub(&elapsed) {
            if duration.num_seconds() < 0 {
                return None;
            }
            return Some(duration);
        }
        None
    }

    /// Elapsed time since last click.
    fn elapsed(&self) -> Duration {
        let now: DateTime<FixedOffset> = Utc::now().into();
        now - self.last_click
    }

    /// Update last click.
    fn update_last_click(&mut self, color: usize, current: Duration) {
        if self.incr == 0 && self.stage == 0 {
            return ;
        }
        let duration = current.checked_add(&self.incr()).unwrap();
        self.clocks[color] = duration;
        self.last_click = Utc::now().into();
    }

    /// Get incr in Duration.
    fn incr(&self) -> Duration {
        Duration::seconds(self.incr)
    }
}

/// Struct used for storing data about players who lost on time.
#[derive(Debug)]
pub struct TimeCheck {
    pub finished: bool,
    pub lost: usize,
    pub both_lost: bool,
    pub id: String,
    pub exist: bool,
}

impl TimeCheck {
    pub fn new(id: &str) -> Self {
        Self {
            finished: false,
            lost: 0,
            both_lost: false,
            id: String::from(id),
            exist: true,
        }
    }
    pub fn finished(&mut self) {
        self.finished = true;
    }

    pub fn both_lost(&mut self) {
        self.both_lost = true;
        self.finished();
    }

    pub fn lost(&mut self, index: usize) {
        self.lost = index;
        self.finished();
    }

    pub fn dont_exist(&mut self) {
        self.exist = false;
    }
}
