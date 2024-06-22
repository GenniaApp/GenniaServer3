use serde::Serialize;
use socketioxide::socket::Sid;

use super::{block::Block, constants::MAX_TEAM_NUM};

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

    pub fn set_spectate(&mut self) {
        (*self).team = MAX_TEAM_NUM + 1;
    }

    pub fn is_spectating(&self) -> bool {
        self.team == MAX_TEAM_NUM + 1
    }
}
