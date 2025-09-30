use ark_ff::PrimeField;
use mpc_algebra_wasm::GroupingParameter;
use nalgebra as na;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Clone)]
pub struct RoleAssignmentPrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub shuffle_matrices: na::DMatrix<F>,
    pub randomness: F::PedersenRandomness,
    pub player_randomness: F,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RoleAssignmentPublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    // parameter
    pub num_players: usize,
    pub max_group_size: usize,
    pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,
    pub grouping_parameter: GroupingParameter,

    // instance
    pub tau_matrix: na::DMatrix<F>,
    pub role_commitment: Vec<F::PedersenCommitment>,
    pub player_commitment: Vec<F::PedersenCommitment>,
}
