use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use crate::{KeyPublicizePrivateInput, KeyPublicizePublicInput};

#[derive(Serialize, Deserialize)]
pub struct KeyPublicizeCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub private_input: Vec<KeyPublicizePrivateInput<F>>,
    pub public_input: KeyPublicizePublicInput<F>,
}
