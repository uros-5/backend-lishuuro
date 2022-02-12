use actix::prelude::{Message, Recipient};

use crate::models::model::ActivePlayer;


#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

#[derive(Message)]
#[rtype(result = "{}")]
pub struct RegularMessage {
    pub text: String,
    pub player: ActivePlayer,
}

impl RegularMessage {
    pub fn new(text: String, username: &String, logged: &bool) -> Self {
        RegularMessage {
            text,
            player: ActivePlayer::new(logged,username)
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub player: ActivePlayer,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub player: ActivePlayer,
}

