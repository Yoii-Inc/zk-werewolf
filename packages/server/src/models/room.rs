use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use super::player::Player;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vote {
    pub target_id: u32,
    pub voters: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub room_id: String,
    pub name: String,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub status: RoomStatus,
    pub roles: Vec<String>,
    pub voting_status: VotingStatus,
    pub votes: HashMap<u32, Vote>, // key: target_player_id, value: Vote
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
            voting_status: VotingStatus::NotStarted,
            votes: HashMap::new(),
        }
    }

    pub fn start_voting(&mut self) {
        self.voting_status = VotingStatus::InProgress;
        self.votes.clear();
    }

    pub fn end_voting(&mut self) {
        self.voting_status = VotingStatus::Completed;
    }

    pub fn reset_voting(&mut self) {
        self.voting_status = VotingStatus::NotStarted;
        self.votes.clear();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VotingStatus {
    NotStarted,
    InProgress,
    Completed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RoomStatus {
    Open,
    InProgress,
    Closed,
}
