use crate::PedersenParam;
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyPublicizePrivateInput {
    pub id: usize,
    pub pub_key_or_dummy_x: Fr,
    pub pub_key_or_dummy_y: Fr,
    pub is_fortune_teller: Fr,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyPublicizePublicInput {
    pub pedersen_param: PedersenParam,
}
