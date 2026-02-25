use super::chat::ChatLog;
use super::player::Player;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoleConfig {
    #[serde(rename = "Seer")]
    pub seer: usize,
    #[serde(rename = "Werewolf")]
    pub werewolf: usize,
    #[serde(rename = "Villager")]
    pub villager: usize,
}

impl RoleConfig {
    pub fn total_players(&self) -> usize {
        self.seer + self.werewolf + self.villager
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeConfig {
    pub day_phase: u64,
    pub night_phase: u64,
    pub voting_phase: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomConfig {
    pub max_players: usize,
    pub role_config: RoleConfig,
    pub time_config: TimeConfig,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            max_players: 9,
            role_config: RoleConfig {
                seer: 1,
                werewolf: 2,
                villager: 6,
            },
            time_config: TimeConfig {
                day_phase: 300,
                night_phase: 120,
                voting_phase: 90,
            },
        }
    }
}

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
    pub room_config: RoomConfig,
    pub status: RoomStatus,
    pub chat_log: ChatLog,
}

impl Room {
    pub fn new(room_id: String, name: Option<String>, room_config: Option<RoomConfig>) -> Self {
        let config = room_config.unwrap_or_default();
        Room {
            room_id: room_id.clone(),
            name,
            players: Vec::new(),
            max_players: config.max_players,
            room_config: config,
            status: RoomStatus::Open,
            chat_log: ChatLog::new(room_id),
        }
    }
}
