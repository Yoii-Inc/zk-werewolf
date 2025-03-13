use serde::{Deserialize, Serialize};

use super::player::Player;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub room_id: String,
    pub name: String,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub status: RoomStatus,
    pub roles: Vec<String>,
}

impl Room {
    pub fn new(room_id: String, name: Option<String>, max_players: Option<usize>) -> Self {
        Room {
            room_id,
            name: name.unwrap_or("".to_string()),
            players: vec![],
            max_players: max_players.unwrap_or(9),
            status: RoomStatus::Open,
            roles: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RoomStatus {
    Open,
    InProgress,
    Closed,
}
