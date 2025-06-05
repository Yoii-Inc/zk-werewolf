use ark_bls12_377::Fr;
use mpc_circuits::*;
use serde::{Deserialize, Serialize};

use crate::{NodeKey, SecretSharingScheme};

#[derive(Serialize, Deserialize)]
pub struct NodeEncryptedShare {
    pub node_id: String,
    pub encrypted_share: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnonymousVotingInput {
    pub private_input: AnonymousVotingPrivateInput<Fr>,
    pub public_input: AnonymousVotingPublicInput<Fr>,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize)]
pub struct AnonymousVotingOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: AnonymousVotingPublicInput<Fr>,
}

#[derive(Serialize, Deserialize)]
pub struct KeyPublicizeInput {
    pub private_input: KeyPublicizePrivateInput<Fr>,
    pub public_input: KeyPublicizePublicInput<Fr>,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize)]
pub struct KeyPublicizeOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: KeyPublicizePublicInput<Fr>,
}

#[derive(Serialize, Deserialize)]
pub struct RoleAssignmentInput {
    pub private_input: RoleAssignmentPrivateInput<Fr>,
    pub public_input: RoleAssignmentPublicInput<Fr>,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize)]
pub struct RoleAssignmentOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: RoleAssignmentPublicInput<Fr>,
}

#[derive(Serialize, Deserialize)]
pub struct DivinationInput {
    pub private_input: DivinationPrivateInput<Fr>,
    pub public_input: DivinationPublicInput<Fr>,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize)]
pub struct DivinationOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: DivinationPublicInput<Fr>,
}

#[derive(Serialize, Deserialize)]
pub struct WinningJudgementInput {
    pub private_input: WinningJudgementPrivateInput<Fr>,
    pub public_input: WinningJudgementPublicInput<Fr>,
    pub node_keys: Vec<NodeKey>,
    pub scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize)]
pub struct WinningJudgementOutput {
    pub shares: Vec<NodeEncryptedShare>,
    pub public_input: WinningJudgementPublicInput<Fr>,
}
