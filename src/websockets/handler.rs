use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension, TypedHeader, headers::UserAgent,
};
use futures::StreamExt;

use crate::database::{redis::UserSession, Database};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<UserAgent>>,
    Extension(db): Extension<Arc<Database>>,
    user: UserSession,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, db, user))
}

async fn websocket(stream: WebSocket, db: Arc<Database>, user: UserSession) {
    println!("{}", &user.username);

    let (mut sender, mut receiver) = stream.split();
    while let Some((Ok(message))) = receiver.next().await {
        if let Message::Text(msg) = message {
            println!("{msg}");
        }
    }
}
