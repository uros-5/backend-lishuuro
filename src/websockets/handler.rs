use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    headers::UserAgent,
    response::IntoResponse,
    Extension, TypedHeader,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::ser;
use serde_json::Value;

use crate::{
    database::{redis::UserSession, Database},
    websockets::{connecting, new_chat_msg, rooms::ChatMsg, ClientMessage, SendTo},
};

use super::{get_chat, get_players, get_players_count, remove_spectator, GameGet, WsState};

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
    user_agent: Option<TypedHeader<UserAgent>>,
    Extension(db): Extension<Arc<Database>>,
    Extension(live): Extension<Arc<WsState>>,
    mut user: UserSession,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, db, live, user))
}

async fn websocket(stream: WebSocket, db: Arc<Database>, ws: Arc<WsState>, mut user: UserSession) {
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
    let rx2 = ws.tx.subscribe();
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
