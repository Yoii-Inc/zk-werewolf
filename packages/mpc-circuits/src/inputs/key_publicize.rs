use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyPublicizePrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub pub_key_or_dummy_x: F,
    pub pub_key_or_dummy_y: F,
    pub is_fortune_teller: F,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyPublicizePublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,
}
