use nalgebra as na;
use serde::{Deserialize, Serialize};

use crate::{GroupingParameter, PedersenCommitment, PedersenParam, PedersenRandomness};
use ark_bn254::Fr;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignmentPrivateInput {
    pub id: usize,
    pub shuffle_matrices: na::DMatrix<Fr>,
    pub randomness: PedersenRandomness,
    pub player_randomness: Fr,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignmentPublicInput {
    // parameter
    pub num_players: usize,
    pub max_group_size: usize,
    pub pedersen_param: PedersenParam,
    pub grouping_parameter: GroupingParameter,

    // instance
    pub tau_matrix: na::DMatrix<Fr>,
    pub role_commitment: Vec<PedersenCommitment>,
    pub player_commitment: Vec<PedersenCommitment>,
}
