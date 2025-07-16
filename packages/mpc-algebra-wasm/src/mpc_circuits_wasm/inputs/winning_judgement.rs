use crate::{PedersenCommitment, PedersenParam};
use ark_bls12_377::Fr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct WinningJudgementPrivateInput {
    pub id: usize,
    pub is_target_id: Vec<Fr>,
    pub player_randomness: Fr,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WinningJudgementPublicInput {
    pub pedersen_param: PedersenParam,
    pub player_commitment: Vec<PedersenCommitment>,
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
