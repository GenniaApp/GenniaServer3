use serde::Serialize;
use socketioxide::socket::Sid;

use super::block::Block;

#[derive(Serialize, Clone, Default, PartialEq)]
pub struct PlayerInRoom {
    pub player_id: String,
    pub username: String,
    pub socket_id: Sid,
    pub color: usize,
    pub team: usize,
    pub is_room_host: bool,
    pub force_start: bool,
    pub is_dead: bool,
    pub last_operate_turn: u32,
    pub land: Vec<Block>,
}

#[derive(Serialize, Default)]
pub struct MinifiedPlayer {
    username: String,
    color: usize,
}

impl PlayerInRoom {
    pub fn minify(&self) -> MinifiedPlayer {
        MinifiedPlayer {
            username: self.username.clone(),
            color: self.color,
        }
    }
}
