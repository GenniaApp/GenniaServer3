mod block;
mod constants;
mod player_in_room;
mod room;

use axum::{http::StatusCode, Json};
use block::Block;
use constants::MAX_TEAM_NUM;
use player_in_room::{MinifiedPlayer, PlayerInRoom};
use prisma_client_rust::QueryError;
use querystring::{querify, QueryParams};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use room::{GameOptions, MinifiedRoom, Room};
use serde::Serialize;
use socketioxide::{
    extract::{Data, SocketRef, State},
    socket::Sid,
};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, VecDeque},
    env,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::prisma::{player, PrismaClient};

#[derive(Serialize)]
pub struct Message {
    from: MinifiedPlayer,
    to: MinifiedPlayer,
    msg: String,
}

pub type RoomPool = BTreeMap<String, Room>;
pub static MAX_ROOM_COUNT: usize = 5;

#[derive(Clone, Default)]
pub struct RoomPoolStore {
    pub pool: Arc<RwLock<RoomPool>>,
}

pub type RoomPoolState = State<RoomPoolStore>;

impl RoomPoolStore {
    pub async fn get(&self) -> RoomPool {
        let binding = self.pool.read().await;
        binding.clone()
    }

    pub async fn serialize(&self) -> Vec<MinifiedRoom> {
        let binding = self.pool.read().await;

        binding
            .iter()
            .map(|(id, room)| room.minify(id.to_string()))
            .collect()
    }

    pub async fn create_room(&self, room_id: String) -> Result<(), &'static str> {
        let mut binding = self.pool.write().await;
        if binding.len() >= MAX_ROOM_COUNT {
            return Err("Room pool length exceeded.");
        }
        let room = Room {
            room_name: "Untitled".to_string(),
            force_start_num: 0,
            game_options: GameOptions {
                max_players: 8,
                game_speed: 1.0,
                map_width: 0.5,
                map_height: 0.5,
                mountain: 0.5,
                city: 0.5,
                swamp: 0.0,
                fog_of_war: true,
                death_spectating: true,
                reveal_king: false,
                warring_state: false,
            },
            game_started: false,
            map_generated: false,
            players: Vec::new(),
            map: Vec::new(),
        };
        let pool = binding.insert(room_id, room);
        Ok(())
    }

    pub async fn find_or_create_room(&self, room_id: String) -> Result<(), &'static str> {
        let binding = self.pool.read().await;
        match binding.clone().get(&room_id) {
            Some(_) => return Ok(()),
            None => return self.create_room(room_id).await,
        }
    }

    pub async fn modify_room_name(&self, room_id: String, name: String) {
        let mut binding = self.pool.write().await;

        if let Some(room) = binding.get_mut(&room_id) {
            (*room).room_name = name;
        }
    }

    pub async fn modify_game_options(&self, room_id: String, options: GameOptions) {
        let mut binding = self.pool.write().await;

        if let Some(room) = binding.get_mut(&room_id) {
            (*room).game_options = options;
        }
    }

    pub async fn add_player(
        &self,
        socket_id: Sid,
        room_id: String,
        player: player::Data,
    ) -> Result<PlayerInRoom, &'static str> {
        let mut binding = self.pool.write().await;

        match binding.get_mut(&room_id) {
            Some(room) => {
                let max_players = (*room).game_options.max_players;
                let player_count = (*room).players.len();
                if max_players > player_count {
                    let mut new_player = PlayerInRoom::default();

                    new_player.username = player.username;
                    new_player.player_id = player.id;
                    new_player.socket_id = socket_id;

                    for i in (1..player_count) {
                        if None == (*room).players.iter().find(|x| x.color == i) {
                            new_player.color = i;
                            break;
                        }
                    }
                    for i in (1..player_count) {
                        if None == (*room).players.iter().find(|x| x.team == i) {
                            new_player.team = i;
                            break;
                        }
                    }

                    (*room).players.push(new_player.clone());

                    return Ok(new_player);
                } else {
                    return Err("Room is full.");
                }
            }
            None => return Err("Room not found."),
        }
    }

    pub async fn modify_player_team(
        &self,
        socket_id: Sid,
        room_id: String,
        team: usize,
    ) -> Result<(), &'static str> {
        let mut binding = self.pool.write().await;

        match binding.get_mut(&room_id) {
            Some(room) => {
                if team > MAX_TEAM_NUM + 1 {
                    return Err("Team number is invalid.");
                }

                for player in (*room).players.iter_mut() {
                    if player.socket_id == socket_id {
                        if team != player.team {
                            player.team = team.clone();

                            if player.is_spectating() && player.force_start {
                                player.force_start = false;
                                room.force_start_num -= 1;
                            }
                        }
                        return Ok(());
                    }
                }
                return Err("Player not found.");
            }
            None => return Err("Room not found."),
        }
    }
}

pub async fn handle_connection(
    socket: SocketRef,
    db: State<Arc<PrismaClient>>,
    room_pool: State<RoomPoolStore>,
) {
    info!("socket connected: {}", socket.id);

    let params = socket
        .req_parts()
        .uri
        .path_and_query()
        .unwrap()
        .query()
        .unwrap();

    let queries = querify(params);

    let username = get_query_param(queries.clone(), "username");
    let player_id = get_query_param(queries.clone(), "player_id");

    match get_player(db, username.clone(), player_id.clone()).await {
        Ok(player) => {
            info!("{} ({}) successfully logged in.", username, player_id);
            let _ = socket.emit("login:success", ());

            socket.on(
                "rooms",
                |socket: SocketRef, room_pool: RoomPoolState| async move {
                    let _ = socket.emit("rooms", room_pool.serialize().await);
                },
            );

            socket.on(
                "create_room",
                |socket: SocketRef, room_pool: RoomPoolState| async move {
                    let room_id = generate_random_string(4);
                    match room_pool.create_room(room_id.clone()).await {
                        Ok(_) => {
                            let _ = socket.emit("create_room:success", room_id);
                        }
                        Err(reason) => {
                            let _ = socket.emit("create_room:failure", reason);
                        }
                    };
                },
            );

            socket.on(
                "join_room",
                |socket: SocketRef,
                 Data::<String>(room_id): Data<String>,
                 room_pool: RoomPoolState| async move {
                    match room_pool.find_or_create_room(room_id.clone()).await {
                        Ok(_) => {
                            match room_pool
                                .add_player(socket.id, room_id.clone(), player.clone())
                                .await
                            {
                                Ok(player_in_room) => {
                                    let _ = socket.leave_all();
                                    let _ = socket.join(room_id.clone());
                                    let _ = socket.emit("join_room:success", room_id.clone());
                                    let _ = socket
                                        .within(room_id.clone())
                                        .emit("message:join", player_in_room.minify());

                                    let pool = room_pool.get().await;
                                    let room = pool.get(&room_id).unwrap();
                                    let _ = socket.within(room_id).emit("room_update", room);
                                }
                                Err(reason) => {
                                    let _ = socket.emit("join_room:failure", reason);
                                }
                            }
                        }
                        Err(reason) => {
                            let _ = socket.emit("join_room:failure", reason);
                        }
                    }
                },
            );

            socket.on(
                "query_room",
                |socket: SocketRef,
                 Data::<String>(room_id): Data<String>,
                 room_pool: RoomPoolState| async move {
                    match room_pool.get().await.get(&room_id) {
                        Some(room) => {
                            let _ = socket.emit("room_update", room);
                        }
                        None => {
                            let _ = socket.emit("query_room:failure", "Room not found.");
                        }
                    }
                },
            );

            socket.on(
                "set_team",
                |socket: SocketRef,
                 Data::<(String, usize)>((room_id, team)): Data<(String, usize)>,
                 room_pool: RoomPoolState| async move {
                    match room_pool.modify_player_team(socket.id, room_id, team).await {
                        Ok(_) => {
                            let _ = socket.emit("set_team:success", ());
                        }
                        Err(reason) => {
                            let _ = socket.emit("set_team:failure", reason);
                        }
                    }
                },
            )
        }
        Err(msg) => {
            let _ = socket.emit("login:failure", msg);
            let _ = socket.disconnect();
            return;
        }
    }
}

async fn get_player(
    db: State<Arc<PrismaClient>>,
    username: String,
    player_id: String,
) -> Result<player::Data, String> {
    if username.starts_with("[Bot]") {
        match db
            .player()
            .find_unique(player::id::equals(player_id.clone()))
            .exec()
            .await
        {
            Ok(data) => {
                let player = data.unwrap();
                if player.clone().username == username {
                    return Ok(player);
                } else {
                    return Err("Username didn't match the player_id.".to_string());
                }
            }
            Err(_) => {
                match db
                    .player()
                    .create(username.clone(), "bot@gennia.online".to_string(), vec![])
                    .exec()
                    .await
                {
                    Ok(data) => {
                        return Ok(data);
                    }
                    Err(err) => {
                        return Err(err.to_string());
                    }
                };
            }
        };
    } else {
        match db
            .player()
            .find_unique(player::id::equals(player_id.clone()))
            .exec()
            .await
        {
            Ok(data) => {
                let player = data.unwrap();
                if player.clone().username == username {
                    return Ok(player);
                } else {
                    return Err("Username didn't match the player_id.".to_string());
                }
            }
            Err(_) => {
                return Err("Player hasn't registered yet.".to_string());
            }
        }
    }
}

fn get_query_param(params: QueryParams, target_key: &str) -> String {
    for (key, value) in params.into_iter() {
        if key == target_key {
            return value.to_string();
        }
    }
    return "".to_string();
}

fn generate_random_string(length: usize) -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    rand_string
}
