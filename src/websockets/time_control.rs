use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::database::mongo::ShuuroGame;
use crate::database::serde_helpers::{array_i32_duration, duration_i32_array};

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

    pub fn update_stage(&mut self, stage: u8) {
        self.stage = stage;
        self.last_click = Utc::now().into();
    }

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

    fn elapsed(&self) -> Duration {
        let now: DateTime<FixedOffset> = Utc::now().into();
        now - self.last_click
    }

    fn update_last_click(&mut self, color: usize, current: Duration) {
        let duration = current.checked_add(&self.incr()).unwrap();
        self.clocks[color] = duration;
        self.last_click = Utc::now().into();
    }

    fn incr(&self) -> Duration {
        Duration::seconds(self.incr)
    }
}
