use super::player::Player;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RoomStatus {
    Open,
    InProgress,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub room_id: String,
    pub name: Option<String>,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub status: RoomStatus,
}

impl Room {
    pub fn new(room_id: String, name: Option<String>, max_players: Option<usize>) -> Self {
        Room {
            room_id,
            name,
            players: Vec::new(),
            max_players: max_players.unwrap_or(9),
            status: RoomStatus::Open,
        }
    }
}
