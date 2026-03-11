use crate::utils::config::CONFIG;
use mpc_algebra_wasm::CircuitEncryptedInputIdentifier;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use zk_mpc_node::{
    models::{ProofOutput, ProofOutputType, ProofRequest, ProofResponse},
    ProofStatus,
};

use crate::{
    models::game::{
        try_convert_to_identifier, BatchEnqueueError, BatchRequest, ClientRequestType, GamePhase,
        GameResult,
    },
    services::proof_job_service::ProofJob,
    state::AppState,
};

const MAX_RETRY_ATTEMPTS: u32 = 180;
const RETRY_DELAY_SECS: u64 = 1;
type BatchExecutionResult = Result<(CircuitEncryptedInputIdentifier, ProofOutput), String>;

static PRECOMPUTED_BATCH_RESULTS: Lazy<Mutex<HashMap<String, BatchExecutionResult>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub enum ProofHandlingError {
    Conflict(String),
    Unprocessable(String),
    Internal(String),
}

/// Sends a request to generate a zero-knowledge proof for the given circuit identifier and inputs.
pub async fn request_proof_with_output(
    circuit_identifier: CircuitEncryptedInputIdentifier,
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
    let _proof_requests = [payload.clone(), payload.clone(), payload.clone()];

    let mut responses = Vec::new();

    for url in CONFIG.zk_mpc_node_urls() {
        let response = client
            .post(url)
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

    if !proof_response.success {
        return Err("Failed to generate proof".to_string());
    }

    Ok(proof_id)
}

pub async fn check_proof_status(proof_id: &str) -> Result<(bool, Option<ProofOutput>), String> {
    let client = Client::new();

    for _ in 0..MAX_RETRY_ATTEMPTS {
        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut all_completed_statuses: Vec<ProofStatus> = Vec::new();

        // 全ノードのステータスをチェック
        for node_url in CONFIG.zk_mpc_node_urls() {
            let response = client
                .get(format!("{}/proof/{}", node_url, proof_id))
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let status: ProofStatus = response.json().await.map_err(|e| e.to_string())?;

            match status.state.as_str() {
                "completed" => {
                    completed_count += 1;
                    all_completed_statuses.push(status);
                }
                "failed" => failed_count += 1,
                _ => continue,
            }
        }

        // 全ノードが完了していたら成功
        if completed_count == CONFIG.zk_mpc_node_urls().len() {
            // 全ノードからの暗号化シェアをマージ
            return Ok((true, merge_proof_outputs(all_completed_statuses)));
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

/// 全ノードからのProofOutputをマージする
/// 各ノードの暗号化シェアを1つのProofOutputに統合
fn merge_proof_outputs(statuses: Vec<ProofStatus>) -> Option<ProofOutput> {
    if statuses.is_empty() {
        return None;
    }

    let mut all_shares = Vec::new();
    let first_output = statuses[0].output.as_ref()?;
    let proof_bytes = statuses.iter().find_map(|status| {
        status
            .output
            .as_ref()
            .and_then(|output| output.proof.clone())
    });
    let public_input_bytes = statuses.iter().find_map(|status| {
        status
            .output
            .as_ref()
            .and_then(|output| output.public_inputs.clone())
    });

    // 各ノードからの暗号化シェアを集める
    for status in &statuses {
        if let Some(output) = &status.output {
            if let Some(shares) = &output.shares {
                println!("Collecting {} shares from node", shares.len());
                all_shares.extend(shares.clone());
            }
        }
    }

    println!("Total merged shares: {}", all_shares.len());

    // マージされたProofOutputを作成
    Some(ProofOutput {
        output_type: first_output.output_type.clone(),
        value: first_output.value.clone(),
        proof: proof_bytes,
        public_inputs: public_input_bytes,
        shares: if all_shares.is_empty() {
            None
        } else {
            Some(all_shares)
        },
    })
}

pub async fn check_status_with_retry(
    proof_id: &str,
) -> Result<(bool, Option<ProofOutput>), String> {
    let (status, output) = check_proof_status(proof_id).await?;
    if status {
        return Ok((true, output));
    }
    println!("Proof ID {} is failed", proof_id);

    Ok((false, None))
}

pub async fn execute_batch_request(batch_request: &BatchRequest) -> BatchExecutionResult {
    let mut sorted_requests = batch_request.requests.clone();
    sorted_requests.sort_by(|a, b| {
        let a_user_id = a.get_user_id().parse::<u32>().unwrap_or(u32::MAX);
        let b_user_id = b.get_user_id().parse::<u32>().unwrap_or(u32::MAX);
        a_user_id.cmp(&b_user_id)
    });

    let identifier = try_convert_to_identifier(sorted_requests.clone())?;

    let output_type = match &identifier {
        CircuitEncryptedInputIdentifier::RoleAssignment(_) => {
            let mut player_pubkeys = Vec::new();
            for request in &sorted_requests {
                if let Some(pubkey) = request.get_public_key() {
                    player_pubkeys.push(zk_mpc_node::UserPublicKey {
                        user_id: request.get_user_id().to_string(),
                        public_key: pubkey.to_string(),
                    });
                }
            }
            if player_pubkeys.is_empty() {
                return Err("No player public keys found for RoleAssignment".to_string());
            }
            ProofOutputType::PrivateToPublic(player_pubkeys)
        }
        _ => ProofOutputType::Public,
    };

    let client = Client::new();
    let req_to_node = ProofRequest {
        proof_id: batch_request.batch_id.clone(),
        circuit_type: identifier.clone(),
        output_type,
    };

    let node_urls = CONFIG.zk_mpc_node_urls();
    let mut responses = Vec::new();
    for url in &node_urls {
        let response = client
            .post(url)
            .json(&req_to_node)
            .send()
            .await
            .map_err(|e| {
                format!(
                    "Failed to send request to {} for batch {}: {}",
                    url, batch_request.batch_id, e
                )
            })?;
        responses.push(response);
    }

    for (url, response) in node_urls.iter().zip(responses) {
        response.json::<serde_json::Value>().await.map_err(|e| {
            format!(
                "Failed to parse JSON response from {} for batch {}: {}",
                url, batch_request.batch_id, e
            )
        })?;
    }

    match check_status_with_retry(&batch_request.batch_id).await? {
        (true, Some(output)) => Ok((identifier, output)),
        (true, None) => Err(format!(
            "Proof completed without output for batch {}",
            batch_request.batch_id
        )),
        (false, _) => Err(format!("Proof failed for batch {}", batch_request.batch_id)),
    }
}

pub async fn store_precomputed_batch_result(batch_id: String, result: BatchExecutionResult) {
    let mut store = PRECOMPUTED_BATCH_RESULTS.lock().await;
    store.insert(batch_id, result);
}

pub async fn take_precomputed_batch_result(batch_id: &str) -> Option<BatchExecutionResult> {
    let mut store = PRECOMPUTED_BATCH_RESULTS.lock().await;
    store.remove(batch_id)
}

pub async fn batch_proof_handling(
    state: AppState,
    room_id: &str,
    request: &ClientRequestType,
) -> Result<String, ProofHandlingError> {
    let (batch_id, proof_job) = {
        let mut games = state.games.lock().await;
        let game = match games.get_mut(room_id) {
            Some(game) => game,
            None => return Err(ProofHandlingError::Internal("Game not found".to_string())),
        };

        validate_phase_for_request(&game.phase, request)?;

        let user_id = match &request {
            ClientRequestType::Divination(info) => info.user_id.clone(),
            ClientRequestType::RoleAssignment(info) => info.user_id.clone(),
            ClientRequestType::AnonymousVoting(info) => info.user_id.clone(),
            ClientRequestType::WinningJudge(info) => info.user_id.clone(),
            ClientRequestType::KeyPublicize(info) => info.user_id.clone(),
        };

        // 計算結果の重複チェック
        match request {
            ClientRequestType::RoleAssignment(_) => {
                if game.has_role_assignment() {
                    return Err(ProofHandlingError::Conflict(
                        "Role assignment has already been completed".to_string(),
                    ));
                }
            }
            ClientRequestType::Divination(_) => {
                if game.has_divination_for_current_phase() {
                    return Err(ProofHandlingError::Conflict(
                        "Divination has already been completed for current phase".to_string(),
                    ));
                }
            }
            ClientRequestType::WinningJudge(_) => {
                // GameResultが既に決定されている場合は重複
                if game.result != GameResult::InProgress {
                    return Err(ProofHandlingError::Conflict(
                        "Winning judgement has already been completed for current phase"
                            .to_string(),
                    ));
                }
            }
            ClientRequestType::AnonymousVoting(_) => {
                // vote_resultsが既に存在し、現在のphaseで投票が完了している場合は重複
                if !game.vote_results.is_empty() {
                    return Err(ProofHandlingError::Conflict(
                        "Voting has already been completed for current phase".to_string(),
                    ));
                }
            }
            ClientRequestType::KeyPublicize(_) => {
                // キー公開は重複チェック対象外
            }
        }

        game.chat_log
            .add_system_message(format!("{} has sent a proof request.", user_id));

        // バッチリクエストに追加
        let enqueue_result = game
            .add_request(request.clone())
            .map_err(|error| match error {
                BatchEnqueueError::Conflict(message) => ProofHandlingError::Conflict(message),
            })?;
        let job = if enqueue_result.should_process {
            Some(ProofJob {
                room_id: room_id.to_string(),
                batch_key: game.build_batch_key(request),
                batch_request: game.batch_request.clone(),
            })
        } else {
            None
        };
        (enqueue_result.batch_id, job)
    };

    if let Some(job) = proof_job {
        state
            .proof_job_service
            .enqueue_job(state.clone(), job)
            .await
            .map_err(ProofHandlingError::Internal)?;
    }

    Ok(batch_id)
}

fn validate_phase_for_request(
    phase: &GamePhase,
    request: &ClientRequestType,
) -> Result<(), ProofHandlingError> {
    let is_valid = match request {
        ClientRequestType::RoleAssignment(_) | ClientRequestType::KeyPublicize(_) => {
            matches!(phase, GamePhase::Night)
        }
        ClientRequestType::Divination(_) => matches!(phase, GamePhase::DivinationProcessing),
        ClientRequestType::AnonymousVoting(_) => matches!(phase, GamePhase::Voting),
        ClientRequestType::WinningJudge(_) => {
            matches!(
                phase,
                GamePhase::DivinationProcessing | GamePhase::Discussion | GamePhase::Result
            )
        }
    };

    if is_valid {
        Ok(())
    } else {
        Err(ProofHandlingError::Unprocessable(format!(
            "phase {:?} does not accept {:?} proof requests",
            phase,
            request.get_proof_type()
        )))
    }
}

// #[derive(Clone)]
// pub struct BatchService {
//     current_batch: Arc<Mutex<Option<BatchRequest>>>,
//     batch_size_limit: usize,
// }

// impl BatchService {
//     pub fn new(batch_size_limit: usize) -> Self {
//         Self {
//             current_batch: Arc::new(Mutex::new(None)),
//             batch_size_limit,
//         }
//     }

//     pub async fn add_request(&self, request: ClientRequestType) -> String {
//         let mut batch_guard = self.current_batch.lock().await;

//         let batch = batch_guard.get_or_insert_with(BatchRequest::new);
//         batch.add_request(request);

//         // バッチが満杯になったら処理を開始
//         if batch.requests.len() >= self.batch_size_limit {
//             let completed_batch = batch_guard.take().unwrap();
//             let batch_id = completed_batch.batch_id.clone();

//             let service = self.clone();

//             // 非同期でバッチ処理を開始
//             tokio::spawn(async move {
//                 service.process_batch(completed_batch).await;
//             });

//             // 新しいバッチを作成
//             *batch_guard = Some(BatchRequest::new());

//             batch_id
//         } else {
//             batch.batch_id.clone()
//         }
//     }

//     async fn process_batch(&self, mut batch: BatchRequest) {
//         batch.status = BatchStatus::Processing;

//         // ここでバッチ処理を実行
//         // 例: ZKプルーフの生成やノードへの送信など

//         batch.status = BatchStatus::Completed;
//     }
// }
