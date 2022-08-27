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
    websockets::{rooms::ChatMsg, SendTo},
};

use super::{GameGet, GameRequest, MessageHandler, WsState};

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

async fn websocket(stream: WebSocket, db: Arc<Database>, ws: Arc<WsState>, user: UserSession) {
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

    let mut recv_task = tokio::spawn(async move {
        let handler = MessageHandler::new(&user, &ws, &tx, &db);
        handler.connecting(true);
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        let data_type = &value["t"];
                        match data_type {
                            serde_json::Value::String(t) => {
                                if t == "live_chat_message" {
                                    if let Ok(mut m) = serde_json::from_str::<ChatMsg>(&text) {
                                        handler.new_chat_msg(&mut m);
                                    }
                                } else if t == "live_chat_full" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        handler.get_chat(m.game_id);
                                    }
                                } else if t == "active_players_full" {
                                    handler.get_players();
                                } else if t == "active_players_count" {
                                    handler.get_players_count();
                                } else if t == "live_game_remove_spectator" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        handler.remove_spectator(&m.game_id);
                                    }
                                } else if t == "home_lobby_add" {
                                    if let Ok(g) = serde_json::from_str::<GameRequest>(&text) {
                                        handler.add_game_req(g);
                                    }
                                } else if t == "home_lobby_full" {
                                    handler.get_all_game_reqs();
                                } else if t == "home_lobby_accept" {
                                    if let Ok(g) = serde_json::from_str::<GameRequest>(&text) {
                                        handler.check_game_req(g).await;
                                    }
                                } else if t == "live_game_hand" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        handler.get_hand(&m.game_id);
                                    }
                                } else if t == "live_game_confirmed" {
                                    if let Ok(m) = serde_json::from_str::<GameGet>(&text) {
                                        handler.get_confirmed(&m.game_id);
                                    }
                                } else if t == "live_game_start" {
                                    if let Ok(g) = serde_json::from_str::<GameGet>(&text) {
                                        handler.get_game(&g.game_id);
                                    }
                                } else if t == "live_game_buy" || t == "live_game_confirm" {
                                    if let Ok(g) = serde_json::from_str::<GameGet>(&text) {
                                        handler.shop_move(g);
                                    }
                                }
                            }
                            _ => println!("{}", &text),
                        }
                    }
                }
                Message::Close(_c) => {
                    handler.connecting(false);
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
