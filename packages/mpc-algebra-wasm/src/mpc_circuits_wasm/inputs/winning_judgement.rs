use crate::{PedersenCommitment, PedersenParam};
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WinningJudgementPrivateInput {
    pub id: usize,
    pub am_werewolf: Fr,
    pub player_randomness: Fr,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WinningJudgementPublicInput {
    pub pedersen_param: PedersenParam,
    pub player_commitment: Vec<PedersenCommitment>,
}
