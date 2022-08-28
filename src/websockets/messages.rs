use std::{collections::HashSet, sync::Arc};

use serde_json::Value;
use tokio::sync::broadcast::Sender;

use crate::database::{
    mongo::ShuuroGame,
    queries::{add_game_to_db, game_exist},
    redis::UserSession,
    Database,
};

use super::{rooms::ChatMsg, GameGet, GameRequest, WsState};

#[derive(Clone)]
pub struct ClientMessage {
    pub reg: bool,
    pub username: String,
    pub msg: Value,
    pub to: SendTo,
}

impl ClientMessage {
    pub fn new(session: &UserSession, msg: Value, to: SendTo) -> Self {
        Self {
            reg: session.reg,
            username: String::from(&session.username),
            msg,
            to,
        }
    }
}

#[derive(Clone)]
pub enum SendTo {
    Me,
    All,
    Spectators(HashSet<String>),
    Players([String; 2]),
    SpectatorsAndPlayers((HashSet<String>, [String; 2])),
}

//Helper functions.
fn fmt_chat(id: &String, chat: Vec<ChatMsg>) -> Value {
    serde_json::json!({"t": "live_chat_full","id": &id, "lines": chat})
}

fn fmt_count(id: &str, cnt: usize) -> Value {
    let id = format!("{}_count", id);
    serde_json::json!({"t": id, "cnt": cnt })
}

pub struct MessageHandler<'a> {
    pub user: &'a UserSession,
    pub ws: &'a Arc<WsState>,
    pub tx: &'a Sender<ClientMessage>,
    pub db: &'a Arc<Database>,
}

impl<'a> MessageHandler<'a> {
    pub fn new(
        user: &'a UserSession,
        ws: &'a Arc<WsState>,
        tx: &'a Sender<ClientMessage>,
        db: &'a Arc<Database>,
    ) -> Self {
        Self { user, ws, tx, db }
    }

    pub fn new_chat_msg(&self, msg: &mut ChatMsg) {
        let id = String::from(&msg.id);
        if let Some(v) = self.ws.chat.add_msg(&id, msg, &self.user) {
            if let Some(s) = self.ws.players.get_spectators(&msg.id) {
                let to: SendTo;
                if &msg.id == "home" {
                    to = SendTo::Spectators(s);
                } else {
                    to = SendTo::SpectatorsAndPlayers((s, [String::from(""), String::from("")]));
                }
                self.send_msg(v, to);
            }
        }
    }

    pub fn get_chat(&self, id: String) {
        if let Some(chat) = self.ws.chat.get_chat(&id) {
            let res = fmt_chat(&id, chat);
            self.send_msg(res, SendTo::Me);
        }
    }

    pub fn get_players(&self) {
        let players = self.ws.players.get_players();
        let res = serde_json::json!({"t": "active_players_full", "players": players});
        self.send_msg(res, SendTo::Me);
    }

    pub fn get_players_count(&self) {
        let res = fmt_count("active_players", self.ws.players.get_players().len());
        self.send_msg(res, SendTo::Me);
    }

    pub fn remove_spectator(&self, id: &String) {
        if let Some(count) = self.ws.players.remove_spectator(id, &self.user.username) {
            let res = fmt_count("live_game_remove_spectator", count);
            if let Some(s) = self.ws.players.get_spectators(&id) {
                let to = SendTo::Spectators(s);
                self.send_msg(res, to);
            }
        }
    }

    pub fn add_game_req(&self, game_req: GameRequest) {
        if let Some(msg) = self.ws.game_reqs.add(game_req) {
            self.send_msg(msg, SendTo::All);
        }
    }

    pub fn get_all_game_reqs(&self) {
        let all = self.ws.game_reqs.get_all();
        let msg = self.ws.game_reqs.response(all);
        self.send_msg(msg, SendTo::Me);
    }

    pub fn remove_game_req(&self, username: &String) {
        if let Some(msg) = self.ws.game_reqs.remove("home_lobby_remove", username) {
            self.send_msg(msg, SendTo::All);
        }
    }

    pub fn shuuro_games_count(&self, to: SendTo) {
        let count = self.ws.shuuro_games.game_count();
        self.send_msg(fmt_count("active_games", count), to);
    }

    async fn accept_game_req(&self, game: GameRequest) {
        let shuuro_game = self.create_game(game).await;
        let msg = add_game_to_db(&self.db.mongo.games, &shuuro_game).await;
        self.send_msg(msg, SendTo::Players(shuuro_game.players.clone()));
        self.ws.players.new_spectators(&shuuro_game._id);
        let _count = self.ws.shuuro_games.add_game(shuuro_game);

        self.shuuro_games_count(SendTo::All);
    }

    pub async fn check_game_req(&self, game: GameRequest) {
        if &game.username() == &self.user.username {
            self.remove_game_req(&game.username);
        } else {
            self.remove_game_req(&game.username);
            self.accept_game_req(game).await;
        }
    }

    pub fn get_hand(&self, id: &String) {
        if let Some(hand) = self.ws.shuuro_games.get_hand(id, &self.user) {
            let msg = serde_json::json!({"t": "live_game_hand", "hand": hand});
            self.send_msg(msg, SendTo::Me);
        }
    }

    pub fn get_confirmed(&self, id: &String) {
        if let Some(confirmed) = self.ws.shuuro_games.get_confirmed(id) {
            let msg = serde_json::json!({"t": "live_game_confirmed", "confirmed": confirmed});
            self.send_msg(msg, SendTo::Me);
        }
    }

    pub fn get_game(&self, id: &String) {
        if let Some(game) = self.ws.shuuro_games.get_game(id) {
            let res =
                serde_json::json!({"t": "live_game_start", "game_id": id, "game_info": &game});
            self.send_msg(res, SendTo::Me);
        }
    }

    fn confirm_shop(&self, json: &GameGet) -> Option<[bool; 2]> {
        if let Some(confirmed) = self.ws.shuuro_games.confirm(&json.game_id, self.user) {
            if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                if let Some(p) = self.ws.shuuro_games.get_players(&json.game_id) {
                    let res = serde_json::json!({"t": "pause_confirmed", "confirmed": confirmed});
                    self.send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
                    return Some(confirmed);
                }
            }
        }
        None
    }

    pub fn shop_move(&self, json: GameGet) {
        if &json.game_move == "cc" {
            if let Some(confirmed) = self.confirm_shop(&json) {
                self.set_deploy(&json, confirmed);
            }
        } else if let Some(confirmed) = self.ws.shuuro_games.buy(&json, &self.user.username) {
            self.confirm_shop(&json);
            self.set_deploy(&json, confirmed);
        }
    }

    pub fn place_move(&self, json: GameGet) {
        if let Some(m) = self.ws.shuuro_games.place_move(&json, &self.user.username) {
            let mut res = serde_json::json!({
                "t": "live_game_place",
                "move": m.0,
                "game_id": &json.game_id,
                "to_fight": false,
                "first_move_error": false,
                "clocks": [140000, 140000]
            });
            self.send_msg(res, SendTo::Players(m.1))
        }
    }

    fn set_deploy(&self, json: &GameGet, confirmed: [bool; 2]) {
        if !confirmed.contains(&false) {
            if let Some(res) = self.ws.shuuro_games.set_deploy(&json.game_id) {
                let s = self.ws.players.get_spectators(&json.game_id).unwrap();
                let p = self.ws.shuuro_games.get_players(&json.game_id).unwrap();
                self.send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
            }
        }
    }

    pub fn connecting(&self, con: bool) {
        let mut _s_count;
        let count: usize = {
            if con {
                self.ws
                    .players
                    .add_spectator(&String::from("home"), &self.user.username);
                self.ws.players.add_player(&self.user.username)
            } else {
                self.ws
                    .players
                    .remove_spectator(&String::from("home"), &self.user.username);
                if let Some(s) = self
                    .ws
                    .players
                    .remove_spectator(&self.user.watches, &self.user.username)
                {
                    _s_count = s;
                }
                if let Some(r) = self
                    .ws
                    .game_reqs
                    .remove("home_lobby_remove", &self.user.username)
                {
                    self.send_msg(r, SendTo::All);
                }
                self.ws.players.remove_player(&self.user.username)
            }
        };
        self.shuuro_games_count(SendTo::Me);
        let value = serde_json::json!({ "t": "active_players_count", "cnt": count });
        self.send_msg(value, SendTo::All);

        if con {
            let chat = self.ws.chat.get_chat(&String::from("home"));
            let value = fmt_chat(&String::from("home"), chat.unwrap());
            self.send_msg(value, SendTo::Me);
        }
    }

    fn send_msg(&self, value: Value, to: SendTo) {
        let cm = ClientMessage::new(self.user, value, to);
        let _ = self.tx.send(cm);
    }
    async fn create_game(&self, game: GameRequest) -> ShuuroGame {
        let colors = game.colors(&self.user.username);
        let id = game_exist(&self.db.mongo.games).await;
        ShuuroGame::from((&game, &colors, id.as_str()))
    }
}
