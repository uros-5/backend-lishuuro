use super::messages::{
    Connect, Disconnect, GameMessage, GameMessageType, RegularMessage, WsMessage,
};
use crate::models::live_games::LiveGames;
use crate::models::model::{
    ActivePlayer, ChatItem, GameGetConfirmed, GameGetHand, GameMove, GameRequest, LobbyGame,
    LobbyGames, ShuuroGame, User,
};
use actix::prelude::{Actor, Context, Handler, Recipient};
use actix::AsyncContext;
use actix::WrapFuture;
use mongodb::Collection;
use serde_json;
use std::collections::HashMap;

type Socket = Recipient<WsMessage>;

#[derive(Clone)]
pub struct Lobby {
    pub chat: Vec<ChatItem>,
    pub active_players: HashMap<ActivePlayer, Socket>,
    pub games: LiveGames,
    pub lobby: LobbyGames,
    pub db_users: Collection<User>,
    pub db_shuuro_games: Collection<ShuuroGame>,
    pub counter: i32,
}

impl Lobby {
    pub fn new(db_users: Collection<User>, db_shuuro_games: Collection<ShuuroGame>) -> Self {
        Lobby {
            chat: vec![],
            active_players: HashMap::new(),
            games: LiveGames::default(),
            lobby: LobbyGames::default(),
            db_users,
            db_shuuro_games,
            counter: 0,
        }
    }

    pub fn send_message(&self, player: &ActivePlayer, message: serde_json::Value) {
        if let Some(socket_recipient) = self.active_players.get(player) {
            let _ = socket_recipient.do_send(WsMessage(message.to_owned().to_string()));
        } else {
        }
    }

    pub fn send_message_to_all(&self, message: serde_json::Value) {
        for user in self.active_players.iter() {
            user.1.do_send(WsMessage(message.to_owned().to_string()));
        }
    }

    pub fn send_message_to_selected(&self, message: serde_json::Value, users: [String; 2]) {
        for user in self.active_players.iter() {
            if users.contains(&&user.0.username()) {
                user.1.do_send(WsMessage(message.to_owned().to_string()));
            }
        }
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;
}

impl Handler<RegularMessage> for Lobby {
    type Result = ();
    //type Result = Future;
    fn handle(&mut self, msg: RegularMessage, ctx: &mut Context<Self>) -> Self::Result {
        let data = serde_json::from_str::<serde_json::Value>(&msg.text);
        let mut res: serde_json::Value = serde_json::json!({"t": "error"});
        match data {
            Ok(i) => {
                let data_type = &i["t"];
                match data_type {
                    serde_json::Value::String(t) => {
                        if t == "home_chat_full" {
                            res = serde_json::json!({"t": t, "lines": self.chat});
                        } else if t == "active_players_count" {
                            res = serde_json::json!({"t": t, "cnt": self.active_players.len()});
                        } else if t == "active_games_count" {
                            res = serde_json::json!({"t": t, "cnt": self.games.shuuro_games.len()});
                        } else if t == "live_game_start" {
                            let m = serde_json::from_str::<GameRequest>(&msg.text);
                            if let Ok(m) = m {
                                let game = self.games.get_game(m.game_id);
                                match game {
                                    Some(g) => {
                                        res = serde_json::json!({"t": "live_game_start", "game_id": &g.0.clone(), "game_info": &g.1});
                                    }
                                    None => (),
                                }
                            }
                        } else if t == "live_game_buy" || t == "live_game_confirm" {
                            let m = serde_json::from_str::<GameMove>(&msg.text);
                            if let Ok(m) = m {
                                self.games
                                    .buy(&m.game_id, m.game_move, &msg.player.username());
                                // if both sides are confirmed then notify them and redirect players.
                                if !self.games.confirmed_players(&m.game_id).contains(&false) {
                                    res = self.games.set_deploy(&m.game_id);
                                    let res2 = serde_json::json!({"t": "pause_confirmed", "confirmed": &self.games.confirmed_players(&m.game_id)});
                                    self.send_message_to_selected(
                                        res2,
                                        self.games.players(&m.game_id),
                                    );
                                    return self.send_message_to_selected(
                                        res,
                                        self.games.players(&m.game_id),
                                    );
                                } else if t == "live_game_confirm" {
                                    res = serde_json::json!({"t": "pause_confirmed", "confirmed": &self.games.confirmed_players(&m.game_id)});
                                    return self.send_message_to_selected(
                                        res,
                                        self.games.players(&m.game_id),
                                    );
                                } else {
                                    return ();
                                }
                            }
                        } else if t == "live_game_place" {
                            let m = serde_json::from_str::<GameMove>(&msg.text);
                            if let Ok(m) = m {
                                let placed = self.games.place(
                                    &m.game_id,
                                    m.game_move,
                                    &msg.player.username(),
                                );
                                if let Some(mut placed) = placed {
                                    *placed.get_mut("game_id").unwrap() =
                                        serde_json::json!(m.game_id);
                                    return self.send_message_to_selected(
                                        placed,
                                        self.games.players(&m.game_id),
                                    );
                                }
                            }
                        } else if t == "live_game_play" {
                            let m = serde_json::from_str::<GameMove>(&msg.text);
                            if let Ok(m) = m {
                                let played = self.games.play(
                                    &m.game_id,
                                    m.game_move,
                                    &msg.player.username(),
                                );
                                if let Some(mut played) = played {
                                    *played.get_mut("game_id").unwrap() =
                                        serde_json::json!(m.game_id);
                                    return self.send_message_to_selected(
                                        played,
                                        self.games.players(&m.game_id),
                                    );
                                }
                            }
                        } else if t == "live_game_hand" {
                            let m = serde_json::from_str::<GameGetHand>(&msg.text);
                            if let Ok(m) = m {
                                let hand = self.games.get_hand(m.game_id, &msg.player.username());
                                res = serde_json::json!({"t": t, "hand": &hand});
                            }
                        } else if t == "live_game_confirmed" {
                            let m = serde_json::from_str::<GameGetConfirmed>(&msg.text);
                            if let Ok(m) = m {
                                let confirmed = self.games.confirmed_players(&m.game_id);
                                res = serde_json::json!({"t": t, "confirmed": &confirmed});
                            }
                        } else if t == "home_chat_message" {
                            let m = serde_json::from_str::<ChatItem>(&msg.text);
                            if let Ok(mut m) = m {
                                m.update(&msg.player.username());
                                if m.message.len() > 0 && m.message.len() < 50 {
                                    // checks also if he posted
                                    res = m.response();
                                    self.chat.push(m);
                                    return self.send_message_to_all(res);
                                }
                            }
                        } else if t == "home_lobby_full" {
                            res = self.lobby.response()
                        } else if t == "just_stop" {
                            let data_type = &i["game_id"];
                            match data_type {
                                serde_json::Value::String(t) => {
                                    self.games.stop(t.clone());
                                }
                                _ => (),
                            }
                        } else if t == "home_lobby_add" {
                            let m = serde_json::from_str::<LobbyGame>(&msg.text);
                            if let Ok(mut game) = m {
                                if game.is_valid() {
                                    if self.lobby.can_add(&game) {
                                        self.games.can_add(&game.username());
                                        if self.games.can_add(&game.username()) {
                                            res = game.response(&t);
                                            self.lobby.add(game);
                                            return self.send_message_to_all(res);
                                        }
                                    }
                                }
                            }
                        } else if t == "home_lobby_accept" {
                            let m = serde_json::from_str::<LobbyGame>(&msg.text);
                            if let Ok(mut game) = m {
                                if game.is_valid() {
                                    if &game.username() == &msg.player.username() {
                                        res = game.response(&String::from("home_lobby_remove"));
                                        let deleted = self.lobby.delete(game);
                                        if deleted >= 0 {
                                            return self.send_message_to_all(res);
                                        }
                                        res = serde_json::json!({"t": "error"});
                                        return self.send_message_to_all(res);
                                    } else {
                                        let users = game.colors(&msg.player.username());
                                        let mut shuuro_game = ShuuroGame::from(&game);
                                        shuuro_game.white = users[0].clone();
                                        shuuro_game.black = users[1].clone();
                                        let res = game.response(&String::from("home_lobby_remove"));
                                        let deleted = self.lobby.delete(game);
                                        if deleted >= 0 {
                                            self.send_message_to_all(res);
                                        }
                                        let deleted = self.lobby.delete_by_user(&msg.player);
                                        if deleted {
                                            let temp_res = serde_json::json!({"t": "home_lobby_remove_user",
                                                "username": &msg.player.username()});
                                            self.send_message_to_all(temp_res);
                                        }
                                        let db_shuuro_games = self.db_shuuro_games.clone();
                                        self.counter += 1;
                                        let ctx2 = ctx.address().clone();
                                        let b = Box::pin(async move {
                                            let game_added =
                                                db_shuuro_games.insert_one(&shuuro_game, None);
                                            match game_added.await {
                                                g => {
                                                    let id =
                                                        g.ok().unwrap().inserted_id.to_string();
                                                    let game_id = id
                                                        .replace("ObjectId(\"", "")
                                                        .replace("\")", "");
                                                    ctx2.do_send(GameMessage {
                                                        message_type:
                                                            GameMessageType::new_adding_game(
                                                                game_id,
                                                                users,
                                                                shuuro_game,
                                                            ),
                                                    });
                                                }
                                            }
                                        });
                                        let actor_future = b.into_actor(self);
                                        ctx.spawn(actor_future);
                                        return;
                                    }
                                }
                            }
                        } else {
                            () //res = serde_json::json!({"t": "error"});
                        }
                    }
                    _ => {
                        () //res = serde_json::json!({"t": "error"});
                    }
                }
            }
            Err(_err) => {
                () //res = serde_json::json!({"t": "error"});
            }
        }

        self.send_message(&msg.player, res)
    }
}

impl Handler<Connect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let user = self.active_players.get(&msg.player);
        match user {
            Some(_i) => {
                ();
            }
            None => {
                let player = msg.player.clone();
                self.active_players.insert(msg.player, msg.addr);
                println!("{}", &player.username());
                self.send_message(
                    &player.clone(),
                    serde_json::json!({"t": "connected","msg": "User connected"}),
                );
            }
        }
        for player in self.active_players.iter() {
            self.send_message(
                &player.0.clone(),
                serde_json::json!({"t": "home_chat_full", "lines": self.chat}),
            );
            self.send_message(
                &player.0.clone(),
                serde_json::json!({"t": "active_players_count", "cnt": self.active_players.len()}),
            );
            self.send_message(
                &player.0.clone(),
                serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()}),
            );
        }
    }
}

impl Handler<Disconnect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        self.active_players.remove(&msg.player);
        self.lobby.delete_by_user(&msg.player);
        let player_count =
            serde_json::json!({"t": "active_players_count", "cnt": self.active_players.len()});
        let matches_count =
            serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
        let temp_res = serde_json::json!({"t": "home_lobby_remove_user",
                                                "username": &msg.player.username()});
        self.send_message_to_all(temp_res);
        self.send_message_to_all(player_count);
        self.send_message_to_all(matches_count);
    }
}

impl Handler<GameMessage> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: GameMessage, ctx: &mut Context<Self>) {
        match msg.message_type {
            GameMessageType::AddingGame {
                game_id,
                users,
                shuuro_game,
            } => {
                let res = serde_json::json!({"t": "live_game_start", "game_id": game_id, "game_info": &shuuro_game });
                self.games.add_game(game_id.clone(), &shuuro_game);
                self.send_message_to_selected(res, users);
                let res = serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
                self.send_message_to_all(res);
            }
        }
    }
}
