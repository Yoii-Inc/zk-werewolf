use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use crate::{DivinationPrivateInput, DivinationPublicInput};

#[derive(Serialize, Deserialize)]
pub struct DivinationCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub private_input: Vec<DivinationPrivateInput<F>>,
    pub public_input: DivinationPublicInput<F>,
}
