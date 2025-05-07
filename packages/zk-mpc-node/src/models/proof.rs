use serde::{Deserialize, Serialize};
use zk_mpc::marlin::MFr;

use super::CircuitIdentifier;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub proof_id: String,
    pub circuit_type: CircuitIdentifier<MFr>,
}
