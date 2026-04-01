use crate::mpc_circuits_wasm::*;
use serde::{Deserialize, Serialize};

use crate::{NodeKey, SecretSharingScheme};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeEncryptedShare {
    pub node_id: String,
    pub encrypted_share: String,
    pub nonce: String,
    pub ephemeral_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousVotingInput {
    pub private_input: AnonymousVotingPrivateInput,
    pub public_input: AnonymousVotingPublicInput,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousVotingOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: AnonymousVotingPublicInput,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPublicizeInput {
    pub private_input: KeyPublicizePrivateInput,
    pub public_input: KeyPublicizePublicInput,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPublicizeOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: KeyPublicizePublicInput,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignmentInput {
    pub private_input: RoleAssignmentPrivateInput,
    pub public_input: RoleAssignmentPublicInput,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignmentOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: RoleAssignmentPublicInput,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivinationInput {
    pub private_input: DivinationPrivateInput,
    pub public_input: DivinationPublicInput,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivinationOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: DivinationPublicInput,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinningJudgementInput {
    pub private_input: WinningJudgementPrivateInput,
    pub public_input: WinningJudgementPublicInput,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinningJudgementOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: WinningJudgementPublicInput,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CircuitEncryptedInputIdentifier {
    Divination(Vec<DivinationOutput>),
    AnonymousVoting(Vec<AnonymousVotingOutput>),
    WinningJudge(Vec<WinningJudgementOutput>),
    RoleAssignment(Vec<RoleAssignmentOutput>),
    KeyPublicize(Vec<KeyPublicizeOutput>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitProfile {
    RoleAssignment {
        player_count: usize,
        werewolf_count: usize,
    },
    Divination {
        player_count: usize,
    },
    AnonymousVoting {
        player_count: usize,
    },
    WinningJudge {
        player_count: usize,
    },
    KeyPublicize {
        player_count: usize,
    },
}

impl CircuitProfile {
    pub fn player_count(&self) -> usize {
        match self {
            Self::RoleAssignment { player_count, .. }
            | Self::Divination { player_count }
            | Self::AnonymousVoting { player_count }
            | Self::WinningJudge { player_count }
            | Self::KeyPublicize { player_count } => *player_count,
        }
    }

    pub fn werewolf_count(&self) -> usize {
        match self {
            Self::RoleAssignment { werewolf_count, .. } => *werewolf_count,
            _ => 0,
        }
    }

    pub fn is_supported_onchain_profile(&self) -> bool {
        match self {
            Self::RoleAssignment {
                player_count: 4,
                werewolf_count: 1,
            } => true,
            Self::RoleAssignment {
                player_count: 5,
                werewolf_count,
            } => *werewolf_count == 1 || *werewolf_count == 2,
            Self::RoleAssignment {
                player_count: 6,
                werewolf_count,
            } => *werewolf_count == 1 || *werewolf_count == 2,
            Self::RoleAssignment {
                player_count: 7,
                werewolf_count,
            } => (1..=3).contains(werewolf_count),
            Self::RoleAssignment {
                player_count: 8,
                werewolf_count,
            } => (1..=3).contains(werewolf_count),
            Self::RoleAssignment {
                player_count: 9,
                werewolf_count,
            } => (1..=3).contains(werewolf_count),
            Self::Divination { player_count } => (3..=9).contains(player_count),
            Self::AnonymousVoting { player_count } => (3..=9).contains(player_count),
            Self::WinningJudge { player_count } => (2..=9).contains(player_count),
            Self::KeyPublicize { player_count } => (4..=9).contains(player_count),
            _ => false,
        }
    }
}

impl CircuitEncryptedInputIdentifier {
    pub fn circuit_profile(&self) -> Option<CircuitProfile> {
        match self {
            CircuitEncryptedInputIdentifier::RoleAssignment(items) => {
                let first = items.first()?;
                Some(CircuitProfile::RoleAssignment {
                    player_count: first.public_input.num_players,
                    werewolf_count: first.public_input.grouping_parameter.get_werewolf_count(),
                })
            }
            CircuitEncryptedInputIdentifier::Divination(items) => {
                let first = items.first()?;
                Some(CircuitProfile::Divination {
                    player_count: first.public_input.player_num,
                })
            }
            CircuitEncryptedInputIdentifier::AnonymousVoting(items) => {
                let first = items.first()?;
                Some(CircuitProfile::AnonymousVoting {
                    player_count: first.public_input.player_num,
                })
            }
            CircuitEncryptedInputIdentifier::WinningJudge(items) => {
                let player_count = items.len();
                Some(CircuitProfile::WinningJudge { player_count })
            }
            CircuitEncryptedInputIdentifier::KeyPublicize(items) => {
                let player_count = items.len();
                Some(CircuitProfile::KeyPublicize { player_count })
            }
        }
    }
}
