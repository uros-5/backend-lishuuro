use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::{Duration};
use serde::ser::SerializeTuple;
use time::{OffsetDateTime, PrimitiveDateTime};

pub fn date_str<S>(x: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date = x.to_string();
    let date = date.split(" +").next().unwrap().clone();
    s.serialize_str(&date)
}

pub fn str_date<'de, D>(data: D) -> Result<OffsetDateTime, D::Error>
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



pub fn duration_i32<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = x.as_millis() as u64;
    s.serialize_u64(duration)
}

pub fn i32_duration<'de, D>(data: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s: u64 = Deserialize::deserialize(data)?;
    let d2 = Duration::from_millis(s);
    Ok(Duration::new(d2.as_secs() as u64, d2.as_nanos() as u32))
}

pub fn clocks_to<S>(x: &(Duration, Duration), s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut tup = s.serialize_tuple(2).ok().unwrap();
    tup.serialize_element(&x.0.as_millis())?;
    tup.serialize_element(&x.1.as_millis())?;
    tup.end()
}

pub fn to_clocks<'de, D>(data: D) -> Result<(Duration, Duration), D::Error>
where
    D: Deserializer<'de>,
{
    let s: [u64; 2] = Deserialize::deserialize(data)?;
    let white = Duration::from_millis(s[0]);
    let black = Duration::from_millis(s[1]);
    Ok((white, black))
}
