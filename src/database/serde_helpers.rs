use async_session::chrono::Duration;
use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serializer};
use std::time::Duration as StdD;

// Serde helpers

/// Serializing from Duration to String
pub fn duration_i32<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = x.num_milliseconds() as u64;
    s.serialize_u64(duration)
}

/// Serializing from String to Duration
pub fn i32_duration<'de, D>(data: D) -> Result<Duration, D::Error>
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
pub fn duration_i32_array<S>(x: &[Duration; 2], s: S) -> Result<S::Ok, S::Error>
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
pub fn array_i32_duration<'de, D>(data: D) -> Result<[Duration; 2], D::Error>
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
