use reqwest::Client;
use tokio::time::{sleep, Duration};
use tracing_subscriber::field::debug;
use zk_mpc::marlin::MFr;
use zk_mpc_node::{
    models::{CircuitIdentifier, ProofOutput, ProofOutputType, ProofRequest, ProofResponse},
    ProofStatus,
};

const ZK_MPC_NODE_URL: [&str; 3] = [
    "http://localhost:9000",
    "http://localhost:9001",
    "http://localhost:9002",
];
const MAX_RETRY_ATTEMPTS: u32 = 30;
const RETRY_DELAY_SECS: u64 = 1;

/// Sends a request to generate a zero-knowledge proof for the given circuit identifier and inputs.
pub async fn request_proof_with_output(
    circuit_identifier: CircuitIdentifier<MFr>,
    output_type: ProofOutputType,
) -> Result<String, String> {
    let client = Client::new();

    let proof_id = uuid::Uuid::new_v4().to_string();

    let payload = ProofRequest {
        proof_id: proof_id.clone(),
        circuit_type: circuit_identifier,
        output_type,
    };

    // TODO: revise individual node payloads
    let proof_requests = [payload.clone(), payload.clone(), payload.clone()];

    let mut responses = Vec::new();

    for port in ZK_MPC_NODE_URL {
        let response = client
            .post(port)
            .json(&payload)
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

    assert!(proof_response.success, "Failed to generate proof");

    Ok(proof_id)
}

pub async fn check_proof_status(proof_id: &str) -> Result<(bool, Option<ProofOutput>), String> {
    let client = Client::new();

    for _ in 0..MAX_RETRY_ATTEMPTS {
        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut last_completed_status: Option<ProofStatus> = None;

        // 全ノードのステータスをチェック
        for node_url in ZK_MPC_NODE_URL.iter() {
            let response = client
                .get(format!("{}/proof/{}", node_url, proof_id))
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let status: ProofStatus = response.json().await.map_err(|e| e.to_string())?;

            match status.state.as_str() {
                "completed" => {
                    completed_count += 1;
                    last_completed_status = Some(status);
                }
                "failed" => failed_count += 1,
                _ => continue,
            }
        }

        // 全ノードが完了していたら成功
        if completed_count == ZK_MPC_NODE_URL.len() {
            return Ok((true, last_completed_status.and_then(|s| s.output)));
        }

        // 1つでも失敗していたら失敗
        if failed_count > 0 {
            return Ok((false, None));
        }
        // まだ完了していないノードがある場合は待機
        sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
    }

    Ok((false, None))
}

pub async fn check_status_with_retry(
    proof_id: &str,
) -> Result<(bool, Option<ProofOutput>), String> {
    for _ in 0..30 {
        let (status, output) = check_proof_status(proof_id).await?;
        if status {
            return Ok((true, output));
        }
        sleep(Duration::from_secs(1)).await;
    }
    Ok((false, None))
}
