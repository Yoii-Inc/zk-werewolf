use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use crate::{AnonymousVotingPrivateInput, AnonymousVotingPublicInput};

#[derive(Serialize, Deserialize)]
pub struct AnonymousVotingCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub private_input: Vec<AnonymousVotingPrivateInput<F>>,
    pub public_input: AnonymousVotingPublicInput<F>,
}
