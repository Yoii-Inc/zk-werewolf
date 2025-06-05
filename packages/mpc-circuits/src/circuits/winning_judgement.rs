use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use crate::{WinningJudgementPrivateInput, WinningJudgementPublicInput};

#[derive(Serialize, Deserialize)]
pub struct WinningJudementCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub private_input: Vec<WinningJudgementPrivateInput<F>>,
    pub public_input: WinningJudgementPublicInput<F>,
}
