use ark_ff::PrimeField;
use serde::{Deserialize, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Clone)]
pub struct WinningJudgementPrivateInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub id: usize,
    pub is_target_id: Vec<F>,
    pub player_randomness: F,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WinningJudgementPublicInput<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,
    pub player_commitment: Vec<<F as LocalOrMPC<F>>::PedersenCommitment>,
}

// #[derive(Serialize, Deserialize)]
// pub struct IndividualWinningJudgementCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
//     pub id: usize,
//     pub total: usize,
//     pub num_alive: F,
//     pub am_werewolf: InputWithCommit<F>,
//     pub game_state: F, // TODO: remove.
//     pub pedersen_param: <F as LocalOrMPC<F>>::PedersenParam,
//     pub player_randomness: F,
//     pub player_commitment: <F as LocalOrMPC<F>>::PedersenCommitment,
// }
