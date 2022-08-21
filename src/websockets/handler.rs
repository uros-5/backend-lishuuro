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
use serde_json::Value;

use crate::{
    database::{redis::UserSession, Database},
    websockets::{ClientMessage, SendTo},
};

use super::WsState;

macro_rules! send_or_break {
    ($sender: expr, $msg: expr, $arr: expr) => {
        if !$arr.len() == 0 {
            if !$arr.contains(&$msg.username) {
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
    user: UserSession,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, db, live, user))
}

async fn websocket(stream: WebSocket, db: Arc<Database>, ws: Arc<WsState>, user: UserSession) {
    let (mut sender, mut receiver) = stream.split();

    let mut rx = ws.tx.subscribe();
    let count = ws.players.add_player(&user.username);
    let username = String::from(&user.username);

    let mut send_task = tokio::spawn(async move {
        let empty = Vec::<String>::new();
        while let Ok(msg) = rx.recv().await {
            match &msg.to {
                SendTo::Me => {
                    if &msg.username == &username {
                        send_or_break!(&mut sender, msg, empty);
                    }
                }
                SendTo::All => {
                    send_or_break!(&mut sender, msg, empty);
                }
                SendTo::Spectators(s) => {
                    send_or_break!(&mut sender, msg, s);
                }
                SendTo::Players(p) => {
                    send_or_break!(&mut sender, msg, p);
                }
                SendTo::SpectatorsAndPlayers(sp) => {
                    if sp.1.contains(&msg.username) {
                        send_or_break!(&mut sender, msg, empty);
                    } else if sp.0.contains(&msg.username) {
                        send_or_break!(&mut sender, msg, empty);
                    }
                }
            }
        }
    });

    let tx = ws.tx.clone();
    let rx2 = ws.tx.subscribe();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(t) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&t) {
                        let msg = ClientMessage::new(&user, value, SendTo::All);
                        tx.send(msg);
                    }
                }
                Message::Close(_c) => {
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
    println!("dead connection");
}
