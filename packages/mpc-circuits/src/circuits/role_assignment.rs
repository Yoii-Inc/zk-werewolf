use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use crate::{RoleAssignmentPrivateInput, RoleAssignmentPublicInput};

#[derive(Serialize, Deserialize)]
pub struct RoleAssignmentCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub private_input: Vec<RoleAssignmentPrivateInput<F>>,
    pub public_input: RoleAssignmentPublicInput<F>,
}
