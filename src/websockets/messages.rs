use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use serde_json::Value;
use tokio::{sync::broadcast::Sender, task::JoinHandle};

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
    rooms::{ChatMsg, Players},
    server_messages::{
        active_players_full, fmt_chat, fmt_count, home_lobby_full,
        live_game_confirmed, live_game_draw, live_game_draw2, live_game_end,
        live_game_hand, live_game_place, live_game_play, live_game_resign,
        live_game_sfen, live_game_start, live_tv, pause_confirmed,
    },
    time_control::TimeCheck,
    GameGet, GameRequest, LiveGameMove, MsgDatabase, WsState,
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

#[derive(Clone)]
pub struct MessageHandler<'a> {
    pub user: &'a UserSession,
    pub ws: &'a Arc<WsState>,
    pub tx: &'a Sender<ClientMessage>,
    pub db: &'a Arc<Database>,
    pub db_tx: &'a Sender<MsgDatabase>,
    pub adding: Arc<Mutex<bool>>,
    pub msg_sender: MsgSender,
}

impl<'a> MessageHandler<'a> {
    pub fn new(
        user: &'a UserSession,
        ws: &'a Arc<WsState>,
        tx: &'a Sender<ClientMessage>,
        db: &'a Arc<Database>,
        db_tx: &'a Sender<MsgDatabase>,
        msg_sender: MsgSender,
    ) -> Self {
        Self {
            user,
            ws,
            tx,
            db,
            db_tx,
            adding: arc2(true),
            msg_sender,
        }
    }

    pub fn new_chat_msg(&self, msg: ChatMsg) {
        let id = String::from(&msg.id);
        let json = GameGet::from(&msg);
        if let Some(v) = self.ws.chat.add_msg(&id, msg, self.user) {
            if let Some(s) = self.ws.players.get_spectators(&id) {
                let to: SendTo;
                if &id == "home" {
                    to = SendTo::Spectators(s);
                } else if let Some(players) =
                    self.ws.shuuro_games.get_players(&json)
                {
                    to = SendTo::SpectatorsAndPlayers((s, players));
                } else {
                    to = SendTo::Spectators(s);
                }
                self.msg_sender.send_msg(v, to);
            }
        }
    }

    pub fn get_chat(&self, id: String) {
        if let Some(chat) = self.ws.chat.get_chat(&id) {
            let res = fmt_chat(&id, chat);
            self.msg_sender.send_msg(res, SendTo::Me);
        }
    }

    pub fn get_players(&self) {
        let players = self.ws.players.get_players();
        let res = active_players_full(players);
        self.msg_sender.send_msg(res, SendTo::Me);
    }

    pub fn get_players_count(&self) {
        let res = fmt_count(
            "active_players_count",
            self.ws.players.get_players().len(),
        );
        self.msg_sender.send_msg(res, SendTo::Me);
    }

    pub fn remove_spectator(&self, id: &String) {
        if let Some(count) =
            self.ws.players.remove_spectator(id, &self.user.username)
        {
            let res = fmt_count("live_game_remove_spectator", count);
            if let Some(s) = self.ws.players.get_spectators(id) {
                let to = SendTo::Spectators(s);
                self.msg_sender.send_msg(res, to);
            }
        }
    }

    pub fn add_spectator(&self, id: &String) {
        if let Some(count) =
            self.ws.players.add_spectator(id, &self.user.username)
        {
            let res = fmt_count("live_game_add_spectator", count);
            if let Some(s) = self.ws.players.get_spectators(id) {
                let to = SendTo::Spectators(s);
                self.msg_sender.send_msg(res, to);
            }
        }
    }

    pub fn add_game_req(&self, game_req: GameRequest) {
        if let Some(msg) = self.ws.game_reqs.add(game_req) {
            self.msg_sender.send_msg(msg, SendTo::All);
        }
    }

    pub fn get_all_game_reqs(&self) {
        let all = self.ws.game_reqs.get_all();
        let msg = home_lobby_full(all);
        self.msg_sender.send_msg(msg, SendTo::Me);
    }

    pub fn remove_game_req(&self, username: &String) {
        if let Some(msg) =
            self.ws.game_reqs.remove("home_lobby_remove", username)
        {
            self.msg_sender.send_msg(msg, SendTo::All);
        }
    }

    pub fn shuuro_games_count(&self, to: SendTo) {
        let count = self.ws.shuuro_games.game_count();
        self.msg_sender
            .send_msg(fmt_count("active_games", count), to);
    }

    async fn accept_game_req(&self, game: GameRequest) {
        let request = game.clone();
        let shuuro_game = self.create_game(game).await;
        let players = shuuro_game.players.clone();
        let id = String::from(&shuuro_game._id);
        let id2 = String::from(&id);
        self.ws.players.new_spectators(&shuuro_game._id);
        let shuuro_game = self.ws.shuuro_games.add_game(shuuro_game, true);
        let msg = add_game_to_db(&self.db.mongo.games, &shuuro_game).await;
        self.msg_sender.send_msg(msg, SendTo::Players(players));
        self.ws
            .shuuro_games
            .change_variant(&GameGet::from((&request, &id2)));
        self.shuuro_games_count(SendTo::All);
        self.ws.chat.add_chat(&id);
        let _lost_on_time_task =
            self.lost_on_time_task(&GameGet::from((&request, &id2)));
        let _check_clock_task = self.check_clock_task(&id);
    }

    pub fn lost_on_time_task(&self, json: &GameGet) -> JoinHandle<()> {
        let mut db_rv = self.db_tx.subscribe();
        let ws2 = self.ws.clone();
        let db = self.db.mongo.games.clone();
        tokio::spawn({
            let json = json.clone();
            let msg_sender = self.msg_sender.clone();
            async move {
                while let Ok(msg) = db_rv.recv().await {
                    if let MsgDatabase::LostOnTime(b) = &msg {
                        ws2.shuuro_games.check_clocks(&json, b);
                        let time_check = b.lock().unwrap();
                        if !time_check.exist {
                            break;
                        } else if time_check.finished {
                            let tv_spectators =
                                ws2.players.get_spectators("tv");
                            let match_spectators =
                                ws2.players.get_spectators(&json.game_id);
                            drop(time_check);
                            if let Some(values) =
                                ws2.shuuro_games.clock_status(&json, b)
                            {
                                msg_sender.send_msg(
                                    values.0.clone(),
                                    SendTo::Players(values.2),
                                );
                                if let Some(s) = match_spectators {
                                    msg_sender.send_msg(
                                        values.0,
                                        SendTo::Spectators(s),
                                    );
                                }
                                msg_sender.send_msg(
                                    values.1,
                                    SendTo::Spectators(tv_spectators.unwrap()),
                                );
                            }

                            tokio::spawn(async move {
                                ws2.shuuro_games
                                    .remove_game(&json, &db.clone())
                                    .await;
                                let count = ws2.shuuro_games.game_count();
                                let msg = fmt_count("active_games", count);
                                msg_sender.send_msg(msg, SendTo::All);
                                ws2.chat.remove_chat(&json.game_id);
                            });
                            break;
                        }
                    }
                }
            }
        })
    }

    pub fn check_clock_task(&self, id: &String) -> JoinHandle<()> {
        let id = String::from(id);
        let db_tx = self.db_tx.clone();
        tokio::spawn(async move {
            let a = arc2(TimeCheck::new(&id));
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let t = a.lock().unwrap();
                if t.finished || t.both_lost || !t.exist {
                    //self2.lost_on_time(&id2, values);
                    break;
                }
                if db_tx.send(MsgDatabase::LostOnTime(a.clone())).is_ok() {}
            }
        })
    }

    pub async fn check_game_req(&self, game: GameRequest) {
        if !*self.adding.lock().unwrap() {
            return;
        }
        if game.username() == self.user.username {
            self.remove_game_req(&game.username);
        } else {
            self.remove_game_req(&game.username);
            self.remove_game_req(&self.user.username);
            self.accept_game_req(game).await;
        }
    }

    pub fn _lost_on_time(&self, json: &GameGet, values: (Value, Value)) {
        if let Some(players) = self.ws.shuuro_games.get_players(json) {
            self.msg_sender
                .send_msg(values.0.clone(), SendTo::Players(players));
            if let Some(spectators) =
                self.ws.players.get_spectators(&json.game_id)
            {
                self.msg_sender
                    .send_msg(values.0, SendTo::Spectators(spectators));
            }
        }
        self.msg_sender.send_tv_msg(values.1, &self.ws.players);
    }

    pub fn get_hand(&self, json: &GameGet) {
        if let Some(hand) = self.ws.shuuro_games.get_hand(json, self.user) {
            let msg = live_game_hand(&hand);
            self.msg_sender.send_msg(msg, SendTo::Me);
        }
    }

    pub fn get_confirmed(&self, json: &GameGet) {
        if let Some(confirmed) = self.ws.shuuro_games.get_confirmed(json) {
            let msg = live_game_confirmed(confirmed);
            self.msg_sender.send_msg(msg, SendTo::Me);
        }
    }

    pub async fn get_game(
        &self,
        json: &GameGet,
        username: &String,
    ) -> Option<String> {
        if let Some(game) = self
            .ws
            .shuuro_games
            .get_game(json, &self.db.mongo.games, self)
            .await
        {
            let res = live_game_start(&game);
            if !&game.players.contains(username) {
                self.ws.players.add_spectator(&game._id, username);
                self.user.watch(&json.game_id);
            }
            self.msg_sender.send_msg(res, SendTo::Me);
            return Some(String::from(&json.game_id));
        }
        None
    }

    fn confirm_shop(&self, json: &GameGet, confirmed: &[bool; 2]) {
        if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
            if let Some(p) = self.ws.shuuro_games.get_players(json) {
                let res = pause_confirmed(confirmed);
                self.msg_sender
                    .send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
            }
        }
    }

    pub fn shop_move(&self, json: GameGet) {
        #[allow(clippy::collapsible_match)]
        if let Some(confirmed) =
            self.ws.shuuro_games.buy(&json, &self.user.username)
        {
            if let LiveGameMove::BuyMove(confirmed) = confirmed {
                self.confirm_shop(&json, &confirmed);
                self.set_deploy(&json, confirmed);
            }
        }
    }

    pub async fn place_move(&self, json: GameGet) {
        #[allow(clippy::collapsible_match)]
        if let Some(m) =
            self.ws.shuuro_games.place_move(&json, &self.user.username)
        {
            if let LiveGameMove::PlaceMove(mv, clocks, fme, tf, p) = m {
                let res = live_game_place(&mv, &json.game_id, tf, fme, &clocks);
                self.msg_sender.send_tv_msg(res.clone(), &self.ws.players);
                if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                    self.msg_sender
                        .send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
                }
                if fme {
                    self.ws
                        .shuuro_games
                        .remove_game(&json, &self.db.mongo.games)
                        .await;
                    self.shuuro_games_count(SendTo::All);
                    self.ws.players.remove_spectators(&json.game_id);
                }
            }
        }
    }

    pub async fn fight_move(&self, json: GameGet) {
        #[allow(clippy::collapsible_match)]
        if let Some(m) =
            self.ws.shuuro_games.fight_move(&json, &self.user.username)
        {
            if let LiveGameMove::FightMove(
                m,
                clocks,
                status,
                _result,
                players,
                o,
            ) = m
            {
                let res =
                    live_game_play(&m, status, &json.game_id, &clocks, &o);
                let tv_res = res.clone();
                if status > 0 {
                    self.ws
                        .shuuro_games
                        .remove_game(&json, &self.db.mongo.games)
                        .await;
                    self.shuuro_games_count(SendTo::All);
                    self.ws.players.remove_spectators(&json.game_id);
                    let res_end = live_game_end(&json.game_id);
                    self.msg_sender.send_tv_msg(res_end, &self.ws.players);
                }
                if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                    self.msg_sender.send_msg(
                        res,
                        SendTo::SpectatorsAndPlayers((s, players)),
                    );
                } else {
                    self.msg_sender.send_msg(res, SendTo::Players(players));
                }
                self.msg_sender.send_tv_msg(tv_res, &self.ws.players);
            }
        }
    }

    fn set_deploy(&self, json: &GameGet, confirmed: [bool; 2]) {
        if !confirmed.contains(&false) {
            if let Some(res) = self.ws.shuuro_games.set_deploy(json) {
                self.msg_sender.send_tv_msg(res.clone(), &self.ws.players);
                let s = self.ws.players.get_spectators(&json.game_id).unwrap();
                let p = self.ws.shuuro_games.get_players(json).unwrap();
                self.msg_sender
                    .send_msg(res, SendTo::SpectatorsAndPlayers((s, p)));
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
                self.ws.players.remove_spectator(
                    &String::from("home"),
                    &self.user.username,
                );
                if let Some(s) = self.ws.players.remove_spectator(
                    &self.user.watches.lock().unwrap(),
                    &self.user.username,
                ) {
                    _s_count = s;
                }
                if let Some(r) = self
                    .ws
                    .game_reqs
                    .remove("home_lobby_remove", &self.user.username)
                {
                    self.msg_sender.send_msg(r, SendTo::All);
                }
                self.ws.players.remove_player(&self.user.username)
            }
        };
        self.shuuro_games_count(SendTo::Me);
        let value = fmt_count("active_players_count", count);
        self.msg_sender.send_msg(value, SendTo::All);

        if con {
            let chat = self.ws.chat.get_chat(&String::from("home"));
            let value = fmt_chat(&String::from("home"), chat.unwrap());
            self.msg_sender.send_msg(value, SendTo::Me);
        } else {
            self.msg_sender
                .send_msg(serde_json::json!({"con": "closed"}), SendTo::Me);
        }
    }

    async fn create_game(&self, game: GameRequest) -> ShuuroGame {
        let colors = game.colors(&self.user.username);
        let id = game_exist(&self.db.mongo.games).await;
        ShuuroGame::from((&game, &colors, id.as_str()))
    }

    // DRAW PART

    pub async fn draw_req(&self, json: &GameGet, username: &String) {
        let draw = self.ws.shuuro_games.draw_req(json, username);
        if let Some(draw) = draw {
            let d = { draw.0 == 5 };

            if draw.0 == 5 {
                let res = live_game_draw(d, &json.game_id);
                self.msg_sender.send_tv_msg(res.clone(), &self.ws.players);
                if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                    self.msg_sender.send_msg(
                        res,
                        SendTo::SpectatorsAndPlayers((s, draw.1)),
                    );
                } else {
                    self.msg_sender.send_msg(res, SendTo::Players(draw.1));
                }
                self.ws
                    .shuuro_games
                    .remove_game(json, &self.db.mongo.games)
                    .await;
                self.shuuro_games_count(SendTo::All);
            } else {
                let res = live_game_draw2(d, &json.game_id, &username);
                if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                    self.msg_sender.send_msg(
                        res,
                        SendTo::SpectatorsAndPlayers((s, draw.1)),
                    );
                } else {
                    self.msg_sender.send_msg(res, SendTo::Players(draw.1));
                }
            }
        }
    }
    pub async fn resign(&self, json: &GameGet, username: &String) {
        if let Some(players) = self.ws.shuuro_games.resign(json, username) {
            let res = live_game_resign(username, &json.game_id);
            self.msg_sender.send_tv_msg(res.clone(), &self.ws.players);
            if let Some(s) = self.ws.players.get_spectators(&json.game_id) {
                self.msg_sender
                    .send_msg(res, SendTo::SpectatorsAndPlayers((s, players)));
            } else {
                self.msg_sender.send_msg(res, SendTo::Players(players));
            }
            self.ws
                .shuuro_games
                .remove_game(json, &self.db.mongo.games)
                .await;
            self.shuuro_games_count(SendTo::All);
        }
    }

    pub fn get_tv(&self) {
        self.remove_spectator(&self.user.watches.lock().unwrap());
        self.add_spectator(&String::from("tv"));
        let all = self.ws.shuuro_games.get_tv();
        let res = live_tv(all);
        self.msg_sender.send_msg(res, SendTo::Me);
    }

    pub fn get_sfen(&self, json: &GameGet) {
        if let Some(g) = self.ws.shuuro_games.live_sfen(json) {
            let res = live_game_sfen(&json.game_id, &g.1, g.0, &json.variant);
            self.msg_sender.send_msg(res, SendTo::Me);
        }
    }

    pub async fn save_all(&self) {
        if self.user.username == "iiiurosiii" {
            *self.adding.lock().unwrap() = false;
            self.ws
                .shuuro_games
                .save_on_exit(&self.db.mongo.games)
                .await;
            std::process::exit(1);
        }
    }

    pub async fn start_unfinished_clock(&self) {
        if self.user.username == "iiiurosiii" {
            let unfinished = self.ws.shuuro_games.get_unfinished();
            if unfinished.is_empty() {
                return;
            }
            let variants =
                ["shuuro", "standard", "shuuroFairy", "standardFairy"];
            for (i, v) in unfinished.iter().enumerate() {
                for id in v {
                    let json = GameGet::new(id, &String::from(variants[i]));
                    let _lost_on_time = self.lost_on_time_task(&json);
                    let _check_clock = self.check_clock_task(id);
                }
            }
            self.ws.shuuro_games.delete_unfinished();
        }
    }
}

#[derive(Clone)]
pub struct MsgSender {
    user: UserSession,
    tx: Sender<ClientMessage>,
}

impl MsgSender {
    pub fn new(user: &UserSession, tx: &Sender<ClientMessage>) -> Self {
        Self {
            user: user.clone(),
            tx: tx.clone(),
        }
    }

    pub fn send_msg(&self, value: Value, to: SendTo) {
        let cm = ClientMessage::new(&self.user, value, to);
        let _ = self.tx.send(cm);
    }

    pub fn send_tv_msg(&self, message: Value, players: &Players) {
        let tv = players.get_spectators("tv").unwrap();
        let message = serde_json::json!({"t": "tv_game_update", "g": message});
        self.send_msg(message, SendTo::Spectators(tv));
    }
}
