use ark_ff::PrimeField;
use nalgebra as na;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Clone)]
pub struct RoleAssignmentPrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub shuffle_matrices: Vec<na::DMatrix<F>>,
    pub randomness: Vec<F::PedersenRandomness>,
    pub player_randomness: Vec<F>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RoleAssignmentPublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    // parameter
    pub num_players: usize,
    pub max_group_size: usize,
    pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,

    // instance
    pub tau_matrix: na::DMatrix<F>,
    pub role_commitment: Vec<F::PedersenCommitment>,
    pub player_commitment: Vec<F::PedersenCommitment>,
}
