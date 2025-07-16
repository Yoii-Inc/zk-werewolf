use crate::{PedersenCommitment, PedersenParam};
use ark_bls12_377::Fr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousVotingPrivateInput {
    pub id: usize,
    pub is_target_id: Vec<Fr>,
    pub player_randomness: Fr,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousVotingPublicInput {
    pub pedersen_param: PedersenParam,
    pub player_commitment: Vec<PedersenCommitment>,
}
