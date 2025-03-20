use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Villager,    // 村人
    Werewolf,    // 人狼
    Seer,        // 占い師
    Medium,      // 霊媒師
    Guard,       // 騎士
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Role::Villager => write!(f, "村人"),
            Role::Werewolf => write!(f, "人狼"),
            Role::Seer => write!(f, "占い師"),
            Role::Medium => write!(f, "霊媒師"),
            Role::Guard => write!(f, "騎士"),
        }
    }
}