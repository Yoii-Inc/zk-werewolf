use super::chat::ChatLog;
use super::player::Player;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RoomStatus {
    Open,
    Ready,
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
    pub chat_log: ChatLog,
}

impl Room {
    pub fn new(room_id: String, name: Option<String>, max_players: Option<usize>) -> Self {
        Room {
            room_id: room_id.clone(),
            name,
            players: Vec::new(),
            max_players: max_players.unwrap_or(9),
            status: RoomStatus::Open,
            chat_log: ChatLog::new(room_id),
        }
    }
}
