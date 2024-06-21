mod block;

use axum::{http::StatusCode, Json};
use block::Block;
use prisma_client_rust::QueryError;
use querystring::{querify, QueryParams};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Serialize;
use socketioxide::extract::{SocketRef, State};
use std::{collections::BTreeMap, env, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;

use crate::prisma::{
    map_diff::data,
    player::{self, email, username},
    PrismaClient,
};

#[derive(Serialize, Clone)]
pub struct GameOptions {
    force_start_num: i16,
    max_players: i16,
    game_speed: f32,
    map_width: f32,
    map_height: f32,
    city: f32,
    swamp: f32,
    mountain: f32,
    fog_of_war: bool,
    death_spectating: bool,
    reveal_king: bool,
    warring_state: bool,
}

#[derive(Serialize, Clone)]
pub struct PlayerInRoom {
    player_id: String,
    username: String,
    socket_id: String,
    color: i16,
    team: i16,
    is_room_host: bool,
    force_start: bool,
    is_dead: bool,
    last_operate_turn: i32,
    land: Block,
}

#[derive(Clone, Serialize)]
pub struct Room {
    room_name: String,
    game_options: GameOptions,
    game_started: bool,
    map_generated: bool,
    players: Vec<PlayerInRoom>,
    map: Vec<Vec<Block>>,
}

pub type RoomPool = BTreeMap<String, Room>;
pub static MAX_ROOM_COUNT: usize = 5;

#[derive(Clone, Default)]
pub struct RoomPoolStore {
    pub pool: Arc<RwLock<RoomPool>>,
}

pub type RoomPoolState = State<RoomPoolStore>;

impl RoomPoolStore {
    pub async fn get_pool(&self) -> RoomPool {
        let binding = self.pool.read().await;
        return binding.clone();
    }

    pub async fn create_room(&self, room_id: String) -> Result<(), &'static str> {
        let mut binding = self.pool.write().await;
        if binding.len() >= MAX_ROOM_COUNT {
            return Err("Room pool length exceeded.");
        }
        let room = Room {
            room_name: "Untitled".to_string(),
            game_options: GameOptions {
                force_start_num: 0,
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
    let mut player_id = get_query_param(queries.clone(), "player_id");
    let room_id = get_query_param(queries.clone(), "room_id");

    if username.starts_with("[Bot]") {
        match db
            .player()
            .find_unique(player::id::equals(player_id.clone()))
            .exec()
            .await
        {
            Ok(data) => {
                if data.unwrap().username == username {
                    let _ = socket.emit("socket_id", socket.id);
                } else {
                    reject_join(socket, "Username didn't match the player_id.").await;
                    return;
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
                        player_id = data.id;
                        let _ = socket.emit("player_id", player_id.clone());
                        let _ = socket.emit("socket_id", socket.id);
                    }
                    Err(err) => {
                        reject_join(socket, err.to_string().as_str()).await;
                        return;
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
                if data.unwrap().username == username {
                    let _ = socket.emit("socket_id", socket.id);
                } else {
                    reject_join(socket, "Username didn't match the player_id.").await;
                    return;
                }
            }
            Err(_) => {
                reject_join(socket, "Player hasn't registered yet.").await;
                return;
            }
        }
    }

    info!(
        "{} ({}) successfully logged in.",
        username.clone(),
        player_id.clone()
    );

    socket.on("rooms", handle_rooms);
    socket.on("create_room", handle_create_room);
}

async fn reject_join(socket: SocketRef, msg: &str) {
    let _ = socket.emit("reject_join", msg);
    let _ = socket.disconnect();
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

async fn handle_rooms(socket: SocketRef, room_pool: RoomPoolState) {
    let _ = socket.emit("rooms", room_pool.get_pool().await);
}

async fn handle_create_room(socket: SocketRef, room_pool: RoomPoolState) {
    let room_id = generate_random_string(4);
    match room_pool.create_room(room_id.clone()).await {
        Ok(_) => {
            let _ = socket.emit("create_room:success", room_id);
        }
        Err(reason) => {
            let _ = socket.emit("create_room:error", reason);
        }
    };
}
