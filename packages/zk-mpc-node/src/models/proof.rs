use serde::{Deserialize, Serialize};
use zk_mpc::marlin::MFr;

use super::{CircuitIdentifier, CircuitInputs};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofStatus {
    pub state: String,
    pub proof_id: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofResponse {
    pub success: bool,
    pub message: String,
    pub proof_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub circuit_type: CircuitIdentifier<MFr>,
    // pub inputs: CircuitInputs,
}
