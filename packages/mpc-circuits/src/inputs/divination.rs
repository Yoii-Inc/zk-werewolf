use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Clone, Serialize, Deserialize)]
pub struct DivinationPrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub is_werewolf: F,
    pub is_target: Vec<F>,
    pub randomness: F::ElGamalRandomness,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DivinationPublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub pedersen_param: F::PedersenParam,
    pub elgamal_param: F::ElGamalParam,
    pub pub_key: F::ElGamalPubKey,
    pub player_num: usize,
}
