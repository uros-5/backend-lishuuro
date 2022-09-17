use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use serde_json::Value;
use tokio::sync::broadcast::Sender;

use crate::{
    arc2,
    database::{
        mongo::ShuuroGame,
        queries::{add_game_to_db, game_exist},
        redis::UserSession,
        Database,
    },
};

use super::{
    rooms::ChatMsg, time_control::TimeCheck, GameGet, GameRequest, LiveGameMove, MsgDatabase,
    WsState,
};

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

#[derive(Clone)]
pub struct MessageHandler<'a> {
    pub user: &'a UserSession,
    pub ws: &'a Arc<WsState>,
    pub tx: &'a Sender<ClientMessage>,
    pub db: &'a Arc<Database>,
    pub db_tx: &'a Sender<MsgDatabase>,
    pub adding: Arc<Mutex<bool>>,
}

impl<'a> MessageHandler<'a> {
    pub fn new(
        user: &'a UserSession,
        ws: &'a Arc<WsState>,
        tx: &'a Sender<ClientMessage>,
        db: &'a Arc<Database>,
        db_tx: &'a Sender<MsgDatabase>,
    ) -> Self {
        Self {
            user,
            ws,
            tx,
            db,
            db_tx,
            adding: arc2(true),
        }
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

    pub fn add_spectator(&self, id: &String) {
        if let Some(count) = self.ws.players.add_spectator(id, &self.user.username) {
            let res = fmt_count("live_game_add_spectator", count);
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
        let id = String::from(&shuuro_game._id);
        let id2 = String::from(&id);
        let msg = add_game_to_db(&self.db.mongo.games, &shuuro_game).await;
        let db = self.db.mongo.games.clone();
        self.send_msg(msg, SendTo::Players(shuuro_game.players.clone()));
        self.ws.players.new_spectators(&shuuro_game._id);
        let _count = self.ws.shuuro_games.add_game(shuuro_game);
        self.shuuro_games_count(SendTo::All);

        let db_tx = self.db_tx.clone();
        let mut db_rv = self.db_tx.subscribe();
        let ws2 = self.ws.clone();

        let db_recv_task = tokio::spawn({
            let session = self.user.clone();
            let tx = self.ws.tx.clone();
            let tv_spectators = self.ws.players.get_spectators("tv");
            let match_spectators = self.ws.players.get_spectators(&id2);
            async move {
                while let Ok(msg) = db_rv.recv().await {
                    match &msg {
                        MsgDatabase::LostOnTime(b) => {
                            ws2.shuuro_games.check_clocks(b);
                            let time_check = b.lock().unwrap();
                            if time_check.exist == false {
                                break;
                            } else if time_check.finished {
                                drop(time_check);
                                if let Some(values) = ws2.shuuro_games.clock_status(b) {
                                    let cm = ClientMessage::new(
                                        &session,
                                        values.0.clone(),
                                        SendTo::Players(values.2),
                                    );
                                    let _ = tx.send(cm);
                                    if let Some(s) = match_spectators {
                                        let cm = ClientMessage::new(
                                            &session,
                                            values.0,
                                            SendTo::Spectators(s),
                                        );
                                        let _ = tx.send(cm);
                                    }

                                    let cm = ClientMessage::new(
                                        &session,
                                        values.1,
                                        SendTo::Spectators(tv_spectators.unwrap()),
                                    );
                                    let _ = tx.send(cm);
                                    let count = ws2.shuuro_games.game_count();
                                    let msg = fmt_count("active_games", count);
                                    let cm = ClientMessage::new(&session, msg, SendTo::All);
                                    let _ = tx.send(cm);
                                }
                                tokio::spawn(async move {
                                    ws2.shuuro_games.remove_game(&db.clone(), &id2).await;
                                });
                                break;
                            }
                        }
                        _ => (),
                    }
                }
            }
        });

        tokio::spawn(async move {
            let a = arc2(TimeCheck::new(&id));
            loop {
                tokio::time::sleep(std::time::Duration::new(2, 0)).await;
                let t = a.lock().unwrap();
                if &t.finished == &true || &t.both_lost == &true || &t.exist == &false {
                    //self2.lost_on_time(&id2, values);
                    break;
                }

                if let Ok(r) = db_tx.send(MsgDatabase::LostOnTime(a.clone())) {}
            }
        });
    }

    pub async fn check_game_req(&self, game: GameRequest) {
        if !*self.adding.lock().unwrap() {
            return;
        }
        if &game.username() == &self.user.username {
            self.remove_game_req(&game.username);
        } else {
            self.remove_game_req(&game.username);
            self.accept_game_req(game).await;
        }
    }

    pub fn lost_on_time(&self, id: &String, values: (Value, Value)) {
        if let Some(players) = self.ws.shuuro_games.get_players(id) {
            self.send_msg(values.0.clone(), SendTo::Players(players));
            if let Some(spectators) = self.ws.players.get_spectators(id) {
                self.send_msg(values.0, SendTo::Spectators(spectators));
            }
        }
        self.send_tv_msg(values.1);
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

    pub async fn get_game(&self, id: &String, username: &String) -> Option<String> {
        if let Some(game) = self
            .ws
            .shuuro_games
            .get_game(id, &self.db.mongo.games, self)
            .await
        {
            let res =
                serde_json::json!({"t": "live_game_start", "game_id": id, "game_info": &game});
            if !&game.players.contains(&username) {
                self.ws.players.add_spectator(id, username);
                self.user.watch(id);
            }
            self.send_msg(res, SendTo::Me);
            return Some(String::from(id));
        }
        None
    }

    fn confirm_shop(&self, json: &GameGet, confirmed: &[bool; 2]) {
        if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
            if let Some(p) = self.ws.shuuro_games.get_players(&json.game_id) {
                let res = serde_json::json!({"t": "pause_confirmed", "confirmed": confirmed});
                self.send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
            }
        }
    }

    pub fn shop_move(&self, json: GameGet) {
        if let Some(confirmed) = self.ws.shuuro_games.buy(&json, &self.user.username) {
            match confirmed {
                LiveGameMove::BuyMove(confirmed) => {
                    self.confirm_shop(&json, &confirmed);
                    self.set_deploy(&json, confirmed);
                }
                _ => (),
            }
        }
    }

    pub fn place_move(&self, json: GameGet) {
        if let Some(m) = self.ws.shuuro_games.place_move(&json, &self.user.username) {
            if let LiveGameMove::PlaceMove(mv, clocks, fme, tf, p) = m {
                let res = serde_json::json!({
                    "t": "live_game_place",
                    "move": mv,
                    "game_id": &json.game_id,
                    "to_fight": tf,
                    "first_move_error": fme,
                    "clocks": clocks
                });
                self.send_tv_msg(res.clone());
                if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                    self.send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
                }
            }
        }
    }

    pub async fn fight_move(&self, json: GameGet) {
        if let Some(m) = self.ws.shuuro_games.fight_move(&json, &self.user.username) {
            if let LiveGameMove::FightMove(m, clocks, status, _result, players, o) = m {
                let res = serde_json::json!({
                    "t": "live_game_play",
                    "game_move": m,
                    "status": status,
                    "game_id": json.game_id,
                    "clocks": clocks,
                    "outcome": o
                });
                let tv_res = res.clone();
                if status > 0 {
                    self.ws
                        .shuuro_games
                        .remove_game(&self.db.mongo.games, &json.game_id)
                        .await;
                    self.shuuro_games_count(SendTo::All);
                    self.ws.players.remove_spectators(&json.game_id);
                    let res_end =
                        serde_json::json!({"t": "live_game_end", "game_id": &json.game_id});
                    self.send_tv_msg(res_end);
                }
                if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                    self.send_msg(res, SendTo::SpectatorsAndPlayers((s, players)));
                } else {
                    self.send_msg(res, SendTo::Players(players));
                }
                self.send_tv_msg(tv_res);
            }
        }
    }

    fn set_deploy(&self, json: &GameGet, confirmed: [bool; 2]) {
        if !confirmed.contains(&false) {
            if let Some(res) = self.ws.shuuro_games.set_deploy(&json.game_id) {
                self.send_tv_msg(res.clone());
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
                    .remove_spectator(&self.user.watches.lock().unwrap(), &self.user.username)
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

    pub fn send_msg(&self, value: Value, to: SendTo) {
        let cm = ClientMessage::new(self.user, value, to);
        let _ = self.tx.send(cm);
    }

    pub fn send_tv_msg(&self, message: Value) {
        let tv = self.ws.players.get_spectators("tv").unwrap();
        let message = serde_json::json!({"t": "tv_game_update", "g": message});
        self.send_msg(message, SendTo::Spectators(tv));
    }

    async fn create_game(&self, game: GameRequest) -> ShuuroGame {
        let colors = game.colors(&self.user.username);
        let id = game_exist(&self.db.mongo.games).await;
        ShuuroGame::from((&game, &colors, id.as_str()))
    }

    // DRAW PART

    pub async fn draw_req(&self, id: &String, username: &String) {
        let draw = self.ws.shuuro_games.draw_req(id, username);
        if let Some(draw) = draw {
            let d = {
                if draw.0 == 5 {
                    true
                } else {
                    false
                }
            };

            if draw.0 == 5 {
                let res = serde_json::json!({"t": "live_game_draw", "draw": d, "game_id": &id});
                self.send_tv_msg(res.clone());
                if let Some(s) = self.ws.players.get_spectators(id) {
                    self.send_msg(res, SendTo::SpectatorsAndPlayers((s, draw.1)));
                } else {
                    self.send_msg(res, SendTo::Players(draw.1));
                }
                self.ws
                    .shuuro_games
                    .remove_game(&self.db.mongo.games, id)
                    .await;
                self.shuuro_games_count(SendTo::All);
            } else {
                let res =
                    serde_json::json!({"t": "live_game_draw", "draw": d, "player": &username});
                if let Some(s) = self.ws.players.get_spectators(id) {
                    self.send_msg(res, SendTo::SpectatorsAndPlayers((s, draw.1)));
                } else {
                    self.send_msg(res, SendTo::Players(draw.1));
                }
            }
        }
    }
    pub async fn resign(&self, id: &String, username: &String) {
        if let Some(players) = self.ws.shuuro_games.resign(id, username) {
            let res = serde_json::json!({
                "t": "live_game_resign",
                "resign": true,
                "player": username,
                "game_id": id
            });
            self.send_tv_msg(res.clone());
            if let Some(s) = self.ws.players.get_spectators(id) {
                self.send_msg(res, SendTo::SpectatorsAndPlayers((s, players)));
            } else {
                self.send_msg(res, SendTo::Players(players));
            }
            self.ws
                .shuuro_games
                .remove_game(&self.db.mongo.games, id)
                .await;
            self.shuuro_games_count(SendTo::All);
        }
    }

    pub fn get_tv(&self) {
        self.remove_spectator(&self.user.watches.lock().unwrap());
        self.add_spectator(&String::from("tv"));
        let all = self.ws.shuuro_games.get_tv();
        let res = serde_json::json!({"t": "live_tv", "games": all});
        self.send_msg(res, SendTo::Me);
    }

    pub fn get_sfen(&self, json: &GameGet) {
        if let Some(g) = self.ws.shuuro_games.live_sfen(&json.game_id) {
            let res = serde_json::json!({
                "t": "live_game_sfen",
                "game_id": &json.game_id,
                "fen": g.1,
                "current_stage": g.0
            });
            self.send_msg(res, SendTo::Me);
        }
    }

    pub async fn save_all(&self, user: &UserSession) {
        if user.username == "iiiurosiii" {
            *self.adding.lock().unwrap() = false;
            self.ws
                .shuuro_games
                .save_on_exit(&self.db.mongo.games)
                .await;
            std::process::exit(1);
        }
    }
}
