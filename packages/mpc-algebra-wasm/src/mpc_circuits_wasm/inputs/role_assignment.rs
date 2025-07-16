use nalgebra as na;
use serde::{Deserialize, Serialize};

use crate::{PedersenCommitment, PedersenParam, PedersenRandomness};
use ark_bls12_377::Fr;

#[derive(Serialize, Deserialize, Clone)]
pub struct RoleAssignmentPrivateInput {
    pub id: usize,
    pub shuffle_matrices: Vec<na::DMatrix<Fr>>,
    pub randomness: Vec<PedersenRandomness>,
    pub player_randomness: Vec<Fr>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RoleAssignmentPublicInput {
    // parameter
    pub num_players: usize,
    pub max_group_size: usize,
    pub pedersen_param: PedersenParam,

    // instance
    pub tau_matrix: na::DMatrix<Fr>,
    pub role_commitment: Vec<PedersenCommitment>,
    pub player_commitment: Vec<PedersenCommitment>,
}
