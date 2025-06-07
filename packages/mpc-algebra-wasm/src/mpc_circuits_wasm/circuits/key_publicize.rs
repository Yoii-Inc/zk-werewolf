use crate::{KeyPublicizePrivateInput, KeyPublicizePublicInput};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct KeyPublicizeCircuit {
    pub private_input: Vec<KeyPublicizePrivateInput>,
    pub public_input: KeyPublicizePublicInput,
}
