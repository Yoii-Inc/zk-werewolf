use crate::{ElGamalParam, ElGamalPubKey, ElGamalRandomness, PedersenParam};
use ark_bls12_377::Fr;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivinationPrivateInput {
    pub id: usize,
    pub is_werewolf: Fr,
    pub is_target: Vec<Fr>,
    pub randomness: ElGamalRandomness,
    // pub randomness_bits: Vec<Fr>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivinationPublicInput {
    pub pedersen_param: PedersenParam,
    pub elgamal_param: ElGamalParam,
    pub pub_key: ElGamalPubKey,
    pub player_num: usize,
}
