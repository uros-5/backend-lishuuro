use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    headers::UserAgent,
    response::IntoResponse,
    TypedHeader,
};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::broadcast;

use crate::{
    database::{queries::get_game_db, redis::UserSession, Database},
    websockets::{rooms::ChatMsg, SendTo},
    AppState,
};

use super::{
    server_messages::live_game_start, ClientMessage, GameGet, GameRequest,
    MessageHandler, MsgDatabase, MsgSender, WsState,
};

macro_rules! send_or_break {
    ($sender: expr, $msg: expr, $username: expr) => {
        if $sender
            .send(Message::Text($msg.msg.to_string()))
            .await
            .is_err()
        {
            let _ = $sender.close().await;
            break;
        }
    };
}

/// Pass all app data to websocket handler.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    _user_agent: Option<TypedHeader<UserAgent>>,
    State(state): State<AppState>,
    user: UserSession,
) -> impl IntoResponse {
    let headers = &user.headers();
    (
        headers.clone(),
        ws.on_upgrade(|socket| websocket(socket, state.db, state.ws, user)),
    )
}

/// Handler for websocket messages.
async fn websocket(
    stream: WebSocket,
    db: Arc<Database>,
    ws: Arc<WsState>,
    user: UserSession,
) {
    let (mut sender, mut receiver) = stream.split();

    let mut rx = ws.tx.subscribe();

    let (db_tx, mut db_rx) = broadcast::channel(100);

    let username = String::from(&user.username);
    let db2 = db.clone();
    let tx2 = ws.tx.clone();
    let user2 = user.clone();

    let mut socket_send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            match &msg.to {
                SendTo::Me => {
                    if msg.username == username {
                        send_or_break!(&mut sender, msg, &username);
                    }
                }
                SendTo::All => {
                    send_or_break!(&mut sender, msg, &username);
                }
                SendTo::Spectators(s) => {
                    if s.contains(&username) {
                        send_or_break!(&mut sender, msg, &username);
                    }
                }
                SendTo::Players(players) => {
                    if players.contains(&username) {
                        send_or_break!(&mut sender, msg, &username);
                    }
                }
                SendTo::SpectatorsAndPlayers(sp) => {
                    if sp.1.contains(&username) {
                        send_or_break!(&mut sender, msg, &username);
                    } else if sp.0.contains(&username) {
                        send_or_break!(&mut sender, msg, &username);
                    }
                }
            }
        }
    });

    let tx = ws.tx.clone();

    let mut socket_recv_task = tokio::spawn(async move {
        let msg_sender = MsgSender::new(&user, &tx);
        let handler =
            MessageHandler::new(&user, &ws, &tx, &db, &db_tx, msg_sender);
        handler.connecting(true);
        handler.start_unfinished_clock().await;
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        let data_type = &value["t"];
                        let data = value["data"].clone();
                        if let serde_json::Value::String(t) = data_type {
                            if t == "live_chat_message" {
                                if let Ok(m) =
                                    serde_json::from_value::<ChatMsg>(data)
                                {
                                    handler.new_chat_msg(m);
                                }
                            } else if t == "live_chat_full" {
                                if let Ok(m) =
                                    serde_json::from_value::<GameGet>(data)
                                {
                                    handler.get_chat(m.game_id);
                                }
                            } else if t == "active_players_full" {
                                handler.get_players();
                            } else if t == "active_players_count" {
                                handler.get_players_count();
                            } else if t == "live_game_remove_spectator" {
                                if let Ok(m) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.remove_spectator(&m.game_id);
                                }
                            } else if t == "home_lobby_add" {
                                if let Ok(g) =
                                    serde_json::from_value::<GameRequest>(data)
                                {
                                    handler.add_game_req(g);
                                }
                            } else if t == "home_lobby_full" {
                                handler.get_all_game_reqs();
                            } else if t == "home_lobby_accept" {
                                if let Ok(g) =
                                    serde_json::from_value::<GameRequest>(data)
                                {
                                    handler.check_game_req(g).await;
                                }
                            } else if t == "live_game_hand" {
                                if let Ok(m) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.get_hand(&m);
                                }
                            } else if t == "live_game_confirmed" {
                                if let Ok(m) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.get_confirmed(&m);
                                }
                            } else if t == "live_game_start" {
                                if let Ok(g) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.get_game(&g, &user.username).await;
                                }
                            } else if t == "live_game_buy"
                                || t == "live_game_confirm"
                            {
                                if let Ok(g) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.shop_move(g);
                                }
                            } else if t == "live_game_place" {
                                if let Ok(g) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.place_move(g).await;
                                }
                            } else if t == "live_game_play" {
                                if let Ok(g) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.fight_move(g).await;
                                }
                            } else if t == "live_game_draw" {
                                if let Ok(g) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.draw_req(&g, &user.username).await;
                                }
                            } else if t == "live_game_resign" {
                                if let Ok(g) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.resign(&g, &user.username).await;
                                }
                            } else if t == "live_game_sfen" {
                                if let Ok(json) =
                                    serde_json::from_str::<GameGet>(&text)
                                {
                                    handler.get_sfen(&json);
                                }
                            } else if t == "live_tv" {
                                handler.get_tv();
                            } else if t == "save_all" {
                                handler.save_all().await;
                            } else {
                            }
                        }
                    }
                }
                Message::Close(_c) => {
                    handler.connecting(false);
                    break;
                }
                _ => handler.connecting(false),
            }
        }
        handler.connecting(false);
    });

    let db_send_task = tokio::spawn(async move {
        while let Ok(msg) = db_rx.recv().await {
            if let MsgDatabase::GetGame(id) = &msg {
                if let Some(game) = get_game_db(&db2.mongo.games, id).await {
                    let msg = live_game_start(&game);
                    let _ =
                        tx2.send(ClientMessage::new(&user2, msg, SendTo::Me));
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut socket_send_task) => {
            socket_recv_task.abort();
            db_send_task.abort();
        },
        _ = (&mut socket_recv_task) => {
            socket_send_task.abort();
            db_send_task.abort();
        }
    }
}
