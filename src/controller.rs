use axum::{
    extract::{Extension}};
use std::sync::{Arc,RwLock};

pub async fn login(Extension(db): Extension<Arc<RwLock<i32>>>) -> String {
    let mut db = db.write().unwrap();
   *db += 1;
    format!("{}", &db)
}

pub async fn callback() -> &'static str {
    "Callback"
}

pub async fn vue_user() -> &'static str {
    "VueUser"
}

pub async fn news() -> &'static str {
    "News"
}
