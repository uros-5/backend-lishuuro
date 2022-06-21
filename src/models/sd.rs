use std::fmt::Debug;

use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::{SerializeTuple}};
use async_session::chrono::{DateTime, Utc, Duration};
pub fn date_str<S>(x: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date =  x.to_rfc3339();
    s.serialize_str(&date)
}

pub fn str_date<'de, D>(data: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(data)?;
    match DateTime::parse_from_rfc3339(s) {
        Ok(d) => {
            return Ok(d.into());
        }
        Err(_e) => {
            return Ok(Utc::now());
        }
    }
}

pub fn duration_i32<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = d_to(x); 
    s.serialize_u64(duration)
}

pub fn i32_duration<'de, D>(data: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: u64 = Deserialize::deserialize(data)?;
    let d2 = to_d(s); 
    Ok(d2)
}

pub fn clocks_i32<S>(x: &[Duration; 2], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration_w = d_to(&x[0]); 
    let duration_b = d_to(&x[1]); 

    let mut a = s.serialize_tuple(2).ok().unwrap();
    a.serialize_element(&duration_w);    
    a.serialize_element(&duration_b);    
    a.end()

}

pub fn i32_clocks<'de, D>(data: D) -> Result<[Duration;2], D::Error>
where
    D: Deserializer<'de>,
{
    let s: [u64;2] = Deserialize::deserialize(data)?;
    let d1 = to_d(s[0]); 
    let d2 = to_d(s[1]); 
    Ok([d1, d2])
}



fn d_to(d: &Duration) -> u64 {
    d.num_milliseconds() as u64
}

fn to_d(s: u64) -> Duration {
    Duration::milliseconds(s as i64)
}