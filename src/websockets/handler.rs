use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    headers::UserAgent,
    response::IntoResponse,
    Extension, TypedHeader,
};
use futures::{SinkExt, StreamExt};
use serde_json::Value;

use crate::{
    database::{redis::UserSession, Database},
    websockets::{connecting, new_chat_msg, rooms::ChatMsg, SendTo},
};

use super::{
    add_game_req, check_game_req, get_all_game_reqs, get_chat, get_confirmed, get_game, get_hand,
    get_players, get_players_count, remove_spectator, GameGet, GameRequest, WsState,
};

macro_rules! send_or_break {
    ($sender: expr, $msg: expr, $arr: expr, $username: expr) => {
        if !$arr.len() == 0 {
            if !$arr.contains($username) {
                return ();
            }
        }
        if $sender
            .send(Message::Text($msg.msg.to_string()))
            .await
            .is_err()
        {
            break;
        }
    };
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    _user_agent: Option<TypedHeader<UserAgent>>,
    Extension(db): Extension<Arc<Database>>,
    Extension(live): Extension<Arc<WsState>>,
    user: UserSession,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, db, live, user))
}

async fn websocket(stream: WebSocket, _db: Arc<Database>, ws: Arc<WsState>, user: UserSession) {
    let (mut sender, mut receiver) = stream.split();

    let mut rx = ws.tx.subscribe();

    //let count = ws.players.add_player(&user.username);
    let username = String::from(&user.username);

    let mut send_task = tokio::spawn(async move {
        let empty = Vec::<String>::new();
        while let Ok(msg) = rx.recv().await {
            match &msg.to {
                SendTo::Me => {
                    if &msg.username == &username {
                        send_or_break!(&mut sender, msg, empty, &username);
                    }
                }
                SendTo::All => {
                    send_or_break!(&mut sender, msg, empty, &username);
                }
                SendTo::Spectators(s) => {
                    send_or_break!(&mut sender, msg, s, &username);
                }
                SendTo::Players(p) => {
                    send_or_break!(&mut sender, msg, p, &username);
                }
                SendTo::SpectatorsAndPlayers(sp) => {
                    if sp.1.contains(&msg.username) {
                        send_or_break!(&mut sender, msg, empty, &username);
                    } else if sp.0.contains(&msg.username) {
                        send_or_break!(&mut sender, msg, empty, &username);
                    }
                }
            }
        }
    });

    let tx = ws.tx.clone();
    connecting(&ws, &user, &ws.tx, true);

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        let data_type = &value["t"];
                        match data_type {
                            serde_json::Value::String(t) => {
                                if t == "live_chat_message" {
                                    if let Ok(mut m) = serde_json::from_str::<ChatMsg>(&text) {
                                        new_chat_msg(&ws, &user, &tx, &mut m);
                                    }
                                } else if t == "live_chat_full" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        get_chat(&ws, &user, &tx, m.game_id);
                                    }
                                } else if t == "active_players_full" {
                                    get_players(&ws, &user, &tx);
                                } else if t == "active_players_count" {
                                    get_players_count(&ws, &user, &tx);
                                } else if t == "live_game_remove_spectator" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        remove_spectator(&ws, &user, &tx, &m.game_id);
                                    }
                                } else if t == "home_lobby_add" {
                                    if let Ok(g) = serde_json::from_str::<GameRequest>(&text) {
                                        add_game_req(&ws, &user, &tx, g);
                                    }
                                } else if t == "home_lobby_full" {
                                    get_all_game_reqs(&ws, &user, &tx);
                                } else if t == "home_lobby_accept" {
                                    if let Ok(g) = serde_json::from_str::<GameRequest>(&text) {
                                        check_game_req(&ws, &_db.mongo.games, &user, &tx, g).await;
                                    }
                                } else if t == "live_game_hand" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        get_hand(&ws, &user, &tx, &m.game_id);
                                    }
                                } else if t == "live_game_confirmed" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        get_confirmed(&ws, &user, &tx, &m.game_id);
                                    }
                                } else if t == "live_game_start" {
                                    if let Ok(g) = serde_json::from_str::<GameGet>(&text) {
                                        get_game(&ws, &user, &tx, &g.game_id);
                                    }
                                } else if t == "live_game_buy" {
                                    if let Ok(g) = serde_json::from_str::<GameGet>(&text) {}
                                }
                            }
                            _ => (),
                        }
                    }
                }
                Message::Close(_c) => {
                    connecting(&ws, &user, &ws.tx, false);
                    break;
                }
                _ => (),
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort()
    }
}
