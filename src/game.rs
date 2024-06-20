mod block;

use std::collections::BTreeMap;

use axum::http::StatusCode;
use block::Block;
use prisma_client_rust::QueryError;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;

fn generate_random_string(length: usize) -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    rand_string
}

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

#[derive(Clone)]
pub struct RoomPool {
    pub pool: BTreeMap<String, Room>,
    pub max_room_count: usize,
}

impl RoomPool {
    pub fn create_room(mut self) -> Result<String, &'static str> {
        if self.pool.len() >= self.max_room_count {
            return Err("Room pool length exceeded.");
        }
        let room_id = generate_random_string(4);
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
        self.pool.insert(room_id.clone(), room);
        Ok(room_id)
    }

    pub async fn modify_room_name(&mut self, room_id: String, name: String) {
        if let Some(room) = self.pool.get_mut(&room_id) {
            (*room).room_name = name.to_string();
        }
    }
}
