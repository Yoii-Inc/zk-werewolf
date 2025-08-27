use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Clone)]
pub struct WinningJudgementPrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub am_werewolf: F,
    pub player_randomness: F,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WinningJudgementPublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,
    pub player_commitment: Vec<<F as LocalOrMPC<F>>::PedersenCommitment>,
}
