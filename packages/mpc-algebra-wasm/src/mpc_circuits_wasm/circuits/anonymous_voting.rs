use crate::{AnonymousVotingPrivateInput, AnonymousVotingPublicInput};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AnonymousVotingCircuit {
    pub private_input: Vec<AnonymousVotingPrivateInput>,
    pub public_input: AnonymousVotingPublicInput,
}
