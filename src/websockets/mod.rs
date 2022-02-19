pub mod client_messages;
pub mod lobby;
pub mod messages;
pub mod start_connection;

use crate::models::model::ActivePlayer;
use actix::prelude::*;
use actix::{fut, ActorContext, ActorFuture, ContextFutureSpawner, WrapFuture};
use actix_web_actors::ws;
use messages::{Connect, Disconnect, RegularMessage, WsMessage};
use std::time::{Duration, Instant};

use self::lobby::Lobby;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsConn {
    hb: Instant,
    username: String,
    logged: bool,
    lobby: Addr<Lobby>,
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        let active_player = ActivePlayer::new(&self.logged, &self.username);

        self.lobby
            .send(Connect {
                addr: addr.recipient(),
                player: active_player,
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => (),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        let active_player = ActivePlayer::new(&self.logged, &self.username);
        self.lobby.do_send(Disconnect {
            player: active_player,
        });
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                println!("here");
                let msg = RegularMessage::new(text, &self.username, &self.logged);
                self.lobby.do_send(msg)
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WsConn {
    fn new(username: String, logged: bool, lobby: Addr<Lobby>) -> Self {
        Self {
            hb: Instant::now(),
            lobby,
            username,
            logged,
        }
    }
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Handler<WsMessage> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        println!("{:?}", &msg.0);
        ctx.text(msg.0);
    }
}
