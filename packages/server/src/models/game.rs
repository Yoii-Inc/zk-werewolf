use serde::{Deserialize, Serialize};

use super::{player::Player, room::RoomStatus};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub room_id: String,
    pub name: String,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub roles: Vec<String>,
}

impl Game {
    pub fn new(room_id: String, players: Vec<Player>) -> Self {
        Game {
            room_id,
            name: "".to_string(),
            players,
            max_players: 9,
            roles: vec![],
        }
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Game {{ room_id: {}, name: {}, players: {:?}, max_players: {}, roles: {:?} }}", 
            self.room_id, self.name, self.players, self.max_players, self.roles)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    Vote(String, bool),
    EndGame,
    NextRole,
    NextTurn,
}
