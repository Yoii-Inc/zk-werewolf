use crate::{DivinationPrivateInput, DivinationPublicInput};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DivinationCircuit {
    pub private_input: Vec<DivinationPrivateInput>,
    pub public_input: DivinationPublicInput,
}
