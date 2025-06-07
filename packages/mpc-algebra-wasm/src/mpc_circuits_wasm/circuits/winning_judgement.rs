use crate::{WinningJudgementPrivateInput, WinningJudgementPublicInput};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WinningJudgementCircuit {
    pub private_input: Vec<WinningJudgementPrivateInput>,
    pub public_input: WinningJudgementPublicInput,
}
