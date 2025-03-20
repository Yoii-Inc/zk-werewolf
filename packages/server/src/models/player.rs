use serde::{Deserialize, Serialize};

use super::role::Role;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub role: Role,
    pub is_dead: bool, // 追加: プレイヤーの生死状態
}

impl Player {
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            role: Role::Villager,
            is_dead: false,
        }
    }
}
