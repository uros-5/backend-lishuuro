use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde_json::Value;

use crate::{database::queries::get_game_db, AppState};

pub fn nuxt() -> Router<AppState> {
    Router::new().route("/shuuro/:id", get(shuuro))
    // .with_state(state)
}

pub async fn shuuro(
    Path(id): Path<String>,
    state: State<AppState>,
) -> Json<Value> {
    let game = get_game_db(&state.db.mongo.games, &id).await;
    if let Some(game) = game {
        Json(
            serde_json::json!({"exist": true, "players": game.players, "result": game.result}),
        )
    } else {
        Json(serde_json::json!({"exist": false}))
    }
    // let players =
}
