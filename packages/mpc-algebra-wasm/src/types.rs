use crate::mpc_circuits_wasm::*;
use serde::{Deserialize, Serialize};

use crate::{NodeKey, SecretSharingScheme};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeEncryptedShare {
    pub node_id: String,
    pub encrypted_share: String,
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
    Divination(DivinationOutput),
    AnonymousVoting(AnonymousVotingOutput),
    WinningJudge(WinningJudgementOutput),
    RoleAssignment(RoleAssignmentOutput),
    KeyPublicize(KeyPublicizeOutput),
}
