use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    Villager,    // 村人
    Werewolf,    // 人狼
    Seer,        // 占い師
    Guard,       // 騎士
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Villager => write!(f, "村人"),
            Role::Werewolf => write!(f, "人狼"),
            Role::Seer => write!(f, "占い師"),
            Role::Guard => write!(f, "騎士"),
        }
    }
}