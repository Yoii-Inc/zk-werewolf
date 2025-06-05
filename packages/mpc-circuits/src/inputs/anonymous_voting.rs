use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Clone)]
pub struct AnonymousVotingPrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub is_target_id: Vec<F>,
    pub player_randomness: F,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AnonymousVotingPublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,
    pub player_commitment: Vec<<F as LocalOrMPC<F>>::PedersenCommitment>,
}
