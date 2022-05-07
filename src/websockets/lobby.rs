use super::messages::{
    Connect, Disconnect, GameMessage, GameMessageType, RegularMessage, WsMessage,
};
use crate::models::db_work::*;
use crate::models::live_games::LiveGames;
use crate::models::model::{
    ActivePlayer, ChatItem, ChatRooms, GameGetConfirmed, GameGetHand, GameMove, GameRequest,
    LobbyGame, LobbyGames, NewsItem, ShuuroGame, User,
};
use actix::prelude::{Actor, Context, Handler, Recipient};
use actix::AsyncContext;
use actix::WrapFuture;
use bson::{doc, oid::ObjectId};
use mongodb::Collection;
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;

type Socket = Recipient<WsMessage>;

#[derive(Clone)]
pub struct Lobby {
    pub chat: Vec<ChatItem>,
    pub chat2: ChatRooms,
    pub active_players: HashMap<ActivePlayer, Socket>,
    pub spectators: HashMap<String, String>,
    pub games: LiveGames,
    pub lobby: LobbyGames,
    pub db_users: Collection<User>,
    pub db_shuuro_games: Collection<ShuuroGame>,
    pub news: Collection<NewsItem>,
    pub counter: i32,
}

impl Lobby {
    pub fn new(
        db_users: Collection<User>,
        db_shuuro_games: Collection<ShuuroGame>,
        news: Collection<NewsItem>,
    ) -> Self {
        Lobby {
            chat: vec![],
            chat2: ChatRooms::new(),
            active_players: HashMap::new(),
            games: LiveGames::default(),
            lobby: LobbyGames::default(),
            db_users,
            db_shuuro_games,
            news,
            spectators: HashMap::new(),
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

    pub fn send_message_to_spectators(&self, game_id: &String, message: serde_json::Value) {
        if let Some(spectators) = self.games.spectators(game_id) {
            for i in spectators.iter() {
                if let Some(user) = self.active_players.get(&ActivePlayer::new(&false, &i)) {
                    user.do_send(WsMessage(message.to_owned().to_string()));
                }
            }
        }
    }

    pub fn send_message_to_tv(&self, message: &serde_json::Value) {
        for (_i, s) in self.spectators.iter().enumerate() {
            if s.1 == "tv" {
                if let Some(user) = self.active_players.get(&ActivePlayer::new(&false, s.0)) {
                    let message = serde_json::json!({"t": "tv_game_update", "g": message});
                    user.do_send(WsMessage(message.to_owned().to_string()));
                }
            }
        }
    }

    pub fn send_message_to_selected(&self, message: serde_json::Value, users: [String; 2]) {
        let mut counter = 0;
        for user in self.active_players.iter() {
            if users.contains(&&user.0.username()) {
                user.1.do_send(WsMessage(message.to_owned().to_string()));
                counter += 1;
                if counter == 2 {
                    break;
                }
            }
        }
    }

    pub fn add_spectator(&mut self, username: &str, game_id: &str) {
        self.spectators
            .insert(String::from(username), String::from(game_id));
    }

    pub fn remove_spectator(&mut self, username: &str) -> (bool, usize) {
        if let Some(game_id) = self.spectators.remove(&String::from(username)) {
            if game_id != String::from("") {
                let count = self.games.remove_spectator(&game_id, username);
                let msg = serde_json::json!({"t": "live_game_spectators_count", "game_id": &game_id, "count": count});
                self.send_message_to_spectators(&game_id, msg);
                return (true, count);
            }
        }
        (false, 0)
    }

    pub fn load_games(&mut self, games: Vec<(String, ShuuroGame)>) -> &mut Self {
        self.games.set_all(games);
        self
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
                        if t == "live_chat_full" {
                            let m = serde_json::from_str::<GameGetConfirmed>(&msg.text);
                            if let Ok(m) = m {
                                if let Some(chat) = self.chat2.chat(&m.game_id) {
                                    res = serde_json::json!({"t": t, "id": &m.game_id, "lines": chat});
                                    return self.send_message(&msg.player, res);
                                }
                            }
                        } else if t == "active_players_count" {
                            res = serde_json::json!({"t": t, "cnt": self.active_players.len()});
                        } else if t == "active_games_count" {
                            res = serde_json::json!({"t": t, "cnt": self.games.shuuro_games.len()});
                        } else if t == "live_tv" {
                            res = serde_json::json!({"t": t, "games": self.games.get_tv()});
                            self.add_spectator(&msg.player.username(), "tv");
                        } else if t == "live_game_remove_spectator" {
                            let m = serde_json::from_str::<GameGetConfirmed>(&msg.text);
                            if let Ok(m) = m {
                                self.remove_spectator(&msg.player.username());
                                let count = self
                                    .games
                                    .remove_spectator(&m.game_id, &msg.player.username());
                                let res = serde_json::json!({"t": "live_game_spectators_count",
                                    "game_id": &m.game_id,
                                    "count": count});
                                return self.send_message_to_spectators(&m.game_id, res);
                            }
                        } else if t == "home_news" {
                            let b = get_home_news(&ctx, &self.news, &msg.player);
                            let actor_future = b.into_actor(self);
                            ctx.spawn(actor_future);
                        } else if t == "live_game_start" {
                            let m = serde_json::from_str::<GameRequest>(&msg.text);
                            if let Ok(m) = m {
                                let game = self.games.get_game(&m.game_id);
                                match game {
                                    Some(g) => {
                                        res = serde_json::json!({"t": "live_game_start", "game_id": &g.0.clone(), "game_info": &g.1});
                                        let spectators = self
                                            .games
                                            .add_spectator(&g.0, &msg.player.username().as_str());
                                        self.add_spectator(
                                            &msg.player.username().as_str(),
                                            &g.0.as_str(),
                                        );
                                        self.send_message(&msg.player, res);
                                        let res_s = serde_json::json!({"t": "live_game_spectators_count", "game_id": &m.game_id, "count": spectators});
                                        return self.send_message_to_spectators(&m.game_id, res_s);
                                    }
                                    None => {
                                        let self2 = self.clone();
                                        let game_id = m.game_id.clone();
                                        let player = msg.player.clone();
                                        let b = get_game(&self2, game_id, player);
                                        let actor_future = b.into_actor(self);
                                        ctx.spawn(actor_future);
                                        return ();
                                    }
                                }
                            }
                        } else if t == "live_game_sfen" {
                            let m = serde_json::from_str::<GameGetConfirmed>(&msg.text);
                            if let Ok(m) = m {
                                if let Some(g) = self.games.get_game(&m.game_id) {
                                    if &g.1.current_stage != &0 {
                                        res = serde_json::json!({"t": t, "game_id": &g.0.clone(), "fen": g.1.sfen, "current_stage": &g.1.current_stage })
                                    }
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
                                    self.send_message_to_spectators(&m.game_id, res2);
                                    self.send_message_to_spectators(&m.game_id, res);
                                    return ();
                                } else if t == "live_game_confirm" {
                                    res = serde_json::json!({"t": "pause_confirmed", "confirmed": &self.games.confirmed_players(&m.game_id)});

                                    self.send_message_to_spectators(&m.game_id, res.clone());
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

                                    self.send_message_to_spectators(&m.game_id, placed.clone());
                                    self.send_message_to_tv(&placed.clone());
                                    //tv
                                    if placed.get("first_move_error").unwrap()
                                        == &serde_json::json!(true)
                                    {
                                        let game = self.games.get_game(&m.game_id).unwrap().1;
                                        let b = update_entire_game(&self, &m.game_id, &game, true);
                                        let actor_future = b.into_actor(self);
                                        ctx.spawn(actor_future);
                                        self.games.remove_game(&m.game_id);
                                        res = serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
                                        let tv_res = serde_json::json!({"t": "live_game_end", "game_id": &m.game_id});
                                        self.send_message_to_tv(&tv_res);
                                        return self.send_message_to_all(res);
                                    }
                                    return ();
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
                                    let status = &played["status"].as_i64().unwrap();

                                    self.send_message_to_spectators(&m.game_id, played.clone());
                                    self.send_message_to_tv(&played.clone());

                                    if status > &0 {
                                        let game = self.games.get_game(&m.game_id).unwrap().1;
                                        let b = update_entire_game(&self, &m.game_id, &game, true);
                                        let actor_future = b.into_actor(self);
                                        ctx.spawn(actor_future);
                                        self.games.remove_game(&m.game_id);
                                        res = serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
                                        let tv_res = serde_json::json!({"t": "live_game_end", "game_id": &m.game_id});
                                        self.send_message_to_tv(&tv_res);
                                        return self.send_message_to_all(res);
                                    }
                                    return ();
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
                        } else if t == "live_game_draw" {
                            let m = serde_json::from_str::<GameGetConfirmed>(&msg.text);
                            if let Ok(m) = m {
                                let draw = self.games.draw_req(&m.game_id, &msg.player.username());
                                if draw == 5 {
                                    res = serde_json::json!({"t": t, "draw": true});
                                    self.send_message_to_spectators(&m.game_id, res);
                                    let game = self.games.get_game(&m.game_id).unwrap().1;
                                    let b = update_entire_game(&self, &m.game_id, &game, true);
                                    let actor_future = b.into_actor(self);
                                    ctx.spawn(actor_future);
                                    self.games.remove_game(&m.game_id);
                                    res = serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
                                    return self.send_message_to_all(res);
                                } else if draw == -2 {
                                    res = serde_json::json!({"t": t, "draw": false, "player": &msg.player.username()});
                                } else if draw == -3 {
                                    return ();
                                }
                                return self.send_message_to_spectators(&m.game_id, res);
                            }
                        } else if t == "live_game_resign" {
                            let m = serde_json::from_str::<GameGetConfirmed>(&msg.text);
                            if let Ok(m) = m {
                                let resign = self.games.resign(&m.game_id, &msg.player.username());
                                if resign {
                                    res = serde_json::json!({"t": t, "resign": true, "player": &msg.player.username()});
                                    self.send_message_to_spectators(&m.game_id, res);
                                    let game = self.games.get_game(&m.game_id).unwrap().1;
                                    let b = update_entire_game(&self, &m.game_id, &game, true);
                                    let actor_future = b.into_actor(self);
                                    ctx.spawn(actor_future);
                                    self.games.remove_game(&m.game_id);
                                    res = serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
                                    return self.send_message_to_all(res);
                                }
                                return ();
                            }
                        } else if t == "live_chat_message" {
                            let m = serde_json::from_str::<ChatItem>(&msg.text);
                            if let Ok(mut m) = m {
                                if let Some(res) =
                                    self.chat2.add_msg(&m.id.clone(), &mut m, &msg.player)
                                {
                                    if &m.id == "home" {
                                        return self.send_message_to_all(res);
                                    } else {
                                        return self.send_message_to_spectators(&m.id.clone(), res);
                                    }
                                }
                            }
                            return ();
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
                                        let db_users = self.db_users.clone();
                                        let b = new_game(
                                            &ctx,
                                            db_shuuro_games,
                                            users,
                                            shuuro_game,
                                            db_users,
                                        );
                                        let actor_future = b.into_actor(self);
                                        ctx.spawn(actor_future);
                                        return;
                                    }
                                }
                            }
                        } else if t == "save_to_db" {
                            if &msg.player.username() == "ADMIN" {
                                res = serde_json::json!({"t": "live_restart"});
                                let all = self.games.get_all();
                                let b = update_all(&self, all);
                                let actor_future = b.into_actor(self);
                                ctx.spawn(actor_future);
                                return self.send_message_to_all(res);
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
                self.send_message(
                    &player.clone(),
                    serde_json::json!({"t": "connected","msg": "User connected"}),
                );
            }
        }
        for player in self.active_players.iter() {
            self.send_message(
                &player.0.clone(),
                serde_json::json!({"t": "live_chat_full", "id": "home", "lines": self.chat2.chat(&String::from("home")).unwrap()}),
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
        let spectator = self.remove_spectator(&msg.player.username().as_str());
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
                mut shuuro_game,
            } => {
                shuuro_game.game_id = game_id.clone();
                self.add_spectator(users[0].as_str(), game_id.as_str());
                self.add_spectator(users[1].as_str(), game_id.as_str());
                let res = serde_json::json!({"t": "live_game_start", "game_id": game_id, "game_info": &shuuro_game });
                self.games.add_game(game_id.clone(), &shuuro_game);
                self.send_message_to_selected(res, users);
                let res = serde_json::json!({"t": "active_games_count", "cnt": self.games.shuuro_games.len()});
                self.send_message_to_all(res);
                self.chat2.add_room(game_id);
            }
            GameMessageType::News {
                news,
                active_player,
            } => {
                let res = serde_json::json!({"t": "home_news", "news": news });
                self.send_message(&active_player, res);
            }
        }
    }
}
