use std::collections::HashSet;

use chrono::Utc;
use serde_json::{json, Value};

use crate::database::mongo::ShuuroGame;

use super::{rooms::ChatMsg, GameRequest, TvGame};

pub fn live_chat_message(msg: &ChatMsg) -> Value {
    json!({ "t": "live_chat_message", "data": msg })
}

pub fn fmt_chat(id: &String, chat: Vec<ChatMsg>) -> Value {
    serde_json::json!({"t": "live_chat_full","data":{ "id": &id, "lines": chat }})
}

pub fn active_players_full(players: HashSet<String>) -> Value {
    serde_json::json!({"t": "active_players_full", "data": { "players": players }})
}

pub fn fmt_count(id: &str, cnt: usize) -> Value {
    let id = format!("{id}_count");
    serde_json::json!({"t": id, "data": { "id": id, "cnt": cnt } })
}

pub fn home_lobby_game(t: &str, game_request: &GameRequest) -> Value {
    serde_json::json!({"t": t, "data": game_request })
}

pub fn home_lobby_full(all: Vec<GameRequest>) -> Value {
    json!({ "t": "home_lobby_full", "data" : { "lobbyGames": all }})
}

pub fn live_game_start(game: &ShuuroGame) -> Value {
    serde_json::json!({"t": "live_game_start", "data": { "game_id": &game._id, "game_info": &game} })
}

pub fn live_game_hand(hand: &str) -> Value {
    serde_json::json!({"t": "live_game_hand", "data": { "hand": hand} })
}

pub fn live_game_confirmed(confirmed: [bool; 2]) -> Value {
    serde_json::json!({"t": "live_game_confirmed", "data":{ "confirmed": confirmed} })
}

pub fn pause_confirmed(confirmed: &[bool; 2]) -> Value {
    serde_json::json!({"t": "pause_confirmed", "data": { "confirmed": confirmed }})
}

pub fn set_deploy(id: &str, hand: &str, game: &ShuuroGame) -> Value {
    serde_json::json!({"t": "redirect_deploy", "data": {
        "path": format!("/shuuro/{id}-1"),
        "hand": hand,
        "last_clock": Utc::now(),
        "side_to_move": "w",
        "w": String::from( &game.players[0]),
        "b": String::from( &game.players[1]),
        "sfen": &game.sfen,
        "variant":  &game.variant
    } })
}

pub fn live_game_place(
    mv: &str,
    game_id: &str,
    tf: bool,
    fme: bool,
    clocks: &[u64; 2],
) -> Value {
    serde_json::json!({"t": "live_game_place",
        "data": {
        "game_move": mv,
        "game_id": game_id,
        "to_fight": tf,
        "first_move_error": fme,
        "clocks": clocks
        }
    })
}

pub fn live_game_play(
    m: &str,
    status: i32,
    game_id: &str,
    clocks: &[u64; 2],
    o: &str,
) -> Value {
    serde_json::json!({
            "t": "live_game_play",
            "data": {
            "game_move": m,
            "status": status,
            "game_id": game_id,
            "clocks": clocks,
            "outcome": o
        }
    })
}

pub fn live_game_draw(d: bool, game_id: &str) -> Value {
    serde_json::json!({"t": "live_game_draw", "data":  {"draw": d, "game_id": game_id}})
}

pub fn live_game_draw2(d: bool, game_id: &str, player: &str) -> Value {
    serde_json::json!({"t": "live_game_draw", "data":  { "draw": d, "player": player, "game_id": game_id}})
}

pub fn live_game_resign(username: &str, game_id: &str) -> Value {
    serde_json::json!({
            "t": "live_game_resign",
    "data": {
            "resign": true,
            "player": username,
            "game_id": game_id
            }
    })
}

pub fn live_game_sfen(
    game_id: &str,
    fen: &str,
    stage: u8,
    variant: &str,
) -> Value {
    serde_json::json!({
        "t": "live_game_sfen",
        "data": {
            "game_id": game_id,
            "fen": fen,
            "current_stage": stage,
            "variant": variant
        }
    })
}

pub fn live_tv(all: Vec<TvGame>) -> Value {
    serde_json::json!({"t": "live_tv", "data": { "games": all}})
}

pub fn live_game_lot(game_id: &str, status: i32, result: &str) -> Value {
    serde_json::json!({
    "t": "live_game_lot",
    "data": {
        "game_id": game_id,
        "status": status,
        "result": String::from(result)
        }
    })
}

pub fn live_game_end(game_id: &str) -> Value {
    serde_json::json!({"t": "live_game_end", "data": { "game_id": String::from(game_id)}})
}
