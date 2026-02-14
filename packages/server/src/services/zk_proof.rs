use crate::utils::config::CONFIG;
use mpc_algebra_wasm::CircuitEncryptedInputIdentifier;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use zk_mpc::marlin::MFr;
use zk_mpc_node::{
    models::{ProofOutput, ProofOutputType, ProofRequest, ProofResponse},
    ProofStatus,
};

use crate::{
    models::game::{BatchRequest, BatchStatus, ClientRequestType},
    state::AppState,
};

const MAX_RETRY_ATTEMPTS: u32 = 180;
const RETRY_DELAY_SECS: u64 = 1;

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
    let proof_requests = [payload.clone(), payload.clone(), payload.clone()];

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

    assert!(proof_response.success, "Failed to generate proof");

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
    let proof_bytes = statuses
        .iter()
        .find_map(|status| status.output.as_ref().and_then(|output| output.proof.clone()));

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

pub async fn batch_proof_handling(
    state: AppState,
    room_id: &str,
    request: &ClientRequestType,
) -> Result<String, String> {
    let mut games = state.games.lock().await;
    let game = match games.get_mut(room_id) {
        Some(game) => game,
        None => return Err("Game not found".to_string()),
    };

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
                return Err("Role assignment has already been completed".to_string());
            }
        }
        ClientRequestType::Divination(_) => {
            if game.has_divination_for_current_phase() {
                return Err("Divination has already been completed for current phase".to_string());
            }
        }
        ClientRequestType::WinningJudge(_) => {
            // GameResultが既に決定されている場合は重複
            if game.result != crate::models::game::GameResult::InProgress {
                return Err(
                    "Winning judgement has already been completed for current phase".to_string(),
                );
            }
        }
        ClientRequestType::AnonymousVoting(_) => {
            // vote_resultsが既に存在し、現在のphaseで投票が完了している場合は重複
            if !game.vote_results.is_empty() {
                return Err("Voting has already been completed for current phase".to_string());
            }
        }
        ClientRequestType::KeyPublicize(_) => {
            // キー公開は重複チェック対象外
        }
    }

    game.chat_log
        .add_system_message(format!("{} has sent a proof request.", user_id));

    // バッチリクエストに追加
    let batch_id = game.add_request(request.clone(), &state).await;

    Ok(batch_id)
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
