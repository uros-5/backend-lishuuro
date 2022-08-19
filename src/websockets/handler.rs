use std::{collections::HashMap, sync::Arc, thread, time::Duration};

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
use tokio::task::JoinHandle;

use crate::database::{redis::UserSession, Database};

use super::WsState;

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

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let tx = ws.tx.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Add username before message.
        }
    });
    println!("dead connection")
}

fn ping(mut sx: SplitSink<WebSocket, Message>) -> JoinHandle<()> {
    let duration = Duration::new(5, 0);
    tokio::spawn(async move {
        loop {
            let msg = vec![];
            if sx.send(Message::Ping(msg)).await.is_err() {
                break;
            }
            thread::sleep(duration);
        }
    })
}

fn da(h: HashMap<String, SplitSink<WebSocket, Message>>) {}
