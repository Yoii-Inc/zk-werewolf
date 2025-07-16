use crate::PedersenParam;
use ark_bls12_377::Fr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyPublicizePrivateInput {
    pub id: usize,
    pub pub_key_or_dummy_x: Vec<Fr>,
    pub pub_key_or_dummy_y: Vec<Fr>,
    pub is_fortune_teller: Vec<Fr>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyPublicizePublicInput {
    pub pedersen_param: PedersenParam,
}
