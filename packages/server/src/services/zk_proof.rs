use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use zk_mpc_node::models::{
    CircuitIdentifier, CircuitInputs, CircuitType, ProofRequest, ProofResponse, ProofStatus,
};

use crate::state::AppState;

const ZK_MPC_NODE_URL: [&str; 3] = [
    "http://localhost:9000",
    "http://localhost:9001",
    "http://localhost:9002",
];
const MAX_RETRY_ATTEMPTS: u32 = 30;
const RETRY_DELAY_SECS: u64 = 1;

/// Sends a request to generate a zero-knowledge proof for the given circuit identifier and inputs.
pub async fn request_proof(
    circuit_identifier: CircuitIdentifier,
    inputs: Value,
) -> Result<String, String> {
    let client = Client::new();

    let payload = ProofRequest {
        circuit_type: circuit_identifier,
        inputs: CircuitInputs::Custom(inputs),
    };

    let proof_requests = [payload.clone(), payload.clone(), payload.clone()];

    let mut responses = Vec::new();

    for port in ZK_MPC_NODE_URL {
        let response = client
            .post(port)
            .json(&proof_requests)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        responses.push(response);
    }

    let proof_response: ProofResponse = responses
        .remove(0)
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(proof_response.proof_id)
}

pub async fn check_proof_status(proof_id: &str) -> Result<bool, String> {
    // if !DEBUG_CONFIG.create_proof {
    //     return Ok(true);
    // }

    // let client = Client::new();

    // for _ in 0..MAX_RETRY_ATTEMPTS {
    //     let response = client
    //         .get(&format!("{}/proof/{}", ZK_MPC_NODE_URL, proof_id))
    //         .send()
    //         .await
    //         .map_err(|e| e.to_string())?;

    //     let status: ProofStatus = response.json().await.map_err(|e| e.to_string())?;

    //     match status.state.as_str() {
    //         "completed" => return Ok(true),
    //         "failed" => return Ok(false),
    //         _ => {
    //             sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
    //             continue;
    //         }
    //     }
    // }

    Ok(false)
}
