use serde::Serialize;

use super::{block::Block, player_in_room::PlayerInRoom};

#[derive(Serialize, Clone)]
pub struct GameOptions {
    pub max_players: usize,
    pub game_speed: f32,
    pub map_width: f32,
    pub map_height: f32,
    pub city: f32,
    pub swamp: f32,
    pub mountain: f32,
    pub fog_of_war: bool,
    pub death_spectating: bool,
    pub reveal_king: bool,
    pub warring_state: bool,
}

#[derive(Clone, Serialize)]
pub struct Room {
    pub room_name: String,
    pub game_options: GameOptions,
    pub force_start_num: usize,
    pub game_started: bool,
    pub map_generated: bool,
    pub players: Vec<PlayerInRoom>,
    pub map: Vec<Vec<Block>>,
}

#[derive(Serialize)]
pub struct MinifiedRoom {
    id: String,
    room_name: String,
    game_started: bool,
    game_speed: f32,
    player_count: usize,
    max_players: usize,
}

impl Room {
    pub fn minify(&self, id: String) -> MinifiedRoom {
        MinifiedRoom {
            id,
            room_name: self.room_name.clone(),
            game_started: self.game_started,
            game_speed: self.game_options.game_speed,
            player_count: self.players.len(),
            max_players: self.game_options.max_players,
        }
    }
}