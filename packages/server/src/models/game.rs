use crate::{
    models::{
        chat::{ChatMessage, ChatMessageType},
        role::Role,
    },
    services::zk_proof::check_status_with_retry,
    utils::config::CONFIG,
};

use super::player::Player;
use ark_bls12_377::Fr;
use ark_crypto_primitives::{encryption::AsymmetricEncryptionScheme, CommitmentScheme};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalDeserialize;
use chrono::{DateTime, Utc};
use derivative::Derivative;
use mpc_algebra_wasm::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};
use zk_mpc_node::{ProofOutputType, ProofRequest};

#[derive(Serialize, Deserialize, Derivative, Clone)]
#[derivative(Debug)]
pub struct Game {
    pub room_id: String,
    pub name: String,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub roles: Vec<String>,
    pub phase: GamePhase,
    pub day_count: u32,
    pub result: GameResult,
    pub night_actions: NightActions,
    pub vote_results: HashMap<String, Vote>,
    pub crypto_parameters: Option<CryptoParameters>,
    pub chat_log: super::chat::ChatLog,
    #[derivative(Debug = "ignore")]
    pub batch_request: BatchRequest,
    pub computation_results: ComputationResults,
    pub started_at: Option<DateTime<Utc>>,
}

// 計算結果を管理する構造体群
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputationResults {
    pub role_assignment: Option<ComputationEntry<RoleAssignmentComputationResult>>,
    pub divination: Vec<ComputationEntry<DivinationComputationResult>>,
}

impl Default for ComputationResults {
    fn default() -> Self {
        ComputationResults {
            role_assignment: None,
            divination: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationEntry<T> {
    pub id: String,
    pub computed_at: DateTime<Utc>,
    pub phase: GamePhase,
    pub day_count: u32,
    pub result: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignmentComputationResult {
    pub player_roles: Vec<PlayerRoleAssignment>,
    pub proof_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRoleAssignment {
    pub player_id: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivinationComputationResult {
    pub ciphertext:
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::Ciphertext,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GamePhase {
    Waiting,              // ゲーム開始前
    Night,                // 夜フェーズ
    DivinationProcessing, // 占い結果処理フェーズ
    Discussion,           // 議論フェーズ
    Voting,               // 投票フェーズ
    Result,               // 結果発表フェーズ
    Finished,             // ゲーム終了
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GameResult {
    InProgress,
    VillagerWin,
    WerewolfWin,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NightAction {
    Attack { target_id: String }, // 人狼の襲撃
    Divine { target_id: String }, // 占い師の占い
    Guard { target_id: String },  // 騎士の護衛
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NightActions {
    pub attacks: Vec<String>,     // 襲撃対象
    pub guards: Vec<String>,      // 護衛対象
    pub divinations: Vec<String>, // 占い対象
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    pub target_id: String,
    pub voters: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NightActionRequest {
    pub player_id: String,
    pub action: NightAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeRoleRequest {
    pub player_id: String,
    pub new_role: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProverInfo {
    pub user_id: String,
    pub prover_count: usize,
    pub encrypted_data: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "proof_type", content = "data")]
pub enum ClientRequestType {
    Divination(ProverInfo),
    AnonymousVoting(ProverInfo),
    WinningJudge(ProverInfo),
    RoleAssignment(ProverInfo),
    KeyPublicize(ProverInfo),
}

impl ClientRequestType {
    fn get_prover_count(&self) -> usize {
        match self {
            ClientRequestType::Divination(info)
            | ClientRequestType::AnonymousVoting(info)
            | ClientRequestType::WinningJudge(info)
            | ClientRequestType::RoleAssignment(info)
            | ClientRequestType::KeyPublicize(info) => info.prover_count,
        }
    }

    fn get_user_id(&self) -> &str {
        match self {
            ClientRequestType::Divination(info)
            | ClientRequestType::AnonymousVoting(info)
            | ClientRequestType::WinningJudge(info)
            | ClientRequestType::RoleAssignment(info)
            | ClientRequestType::KeyPublicize(info) => info.user_id.as_str(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub batch_id: String,
    pub requests: Vec<ClientRequestType>,
    pub status: BatchStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl BatchRequest {
    pub fn new() -> Self {
        BatchRequest {
            batch_id: uuid::Uuid::new_v4().to_string(),
            requests: Vec::new(),
            status: BatchStatus::Collecting,
            created_at: chrono::Utc::now(),
        }
    }
}

fn try_convert_to_identifier(
    requests: Vec<ClientRequestType>,
) -> Result<CircuitEncryptedInputIdentifier, String> {
    use ClientRequestType::*;
    match requests.split_first() {
        Some((first, rest)) => match first {
            Divination(_) if rest.iter().all(|r| matches!(r, Divination(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let Divination(d) = r {
                            serde_json::from_str(&d.encrypted_data)
                                .expect("Failed to deserialize DivinationOutput")
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(CircuitEncryptedInputIdentifier::Divination(items))
            }
            AnonymousVoting(_) if rest.iter().all(|r| matches!(r, AnonymousVoting(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let AnonymousVoting(d) = r {
                            serde_json::from_str(&d.encrypted_data)
                                .expect("Failed to deserialize AnonymousVotingOutput")
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(CircuitEncryptedInputIdentifier::AnonymousVoting(items))
            }
            WinningJudge(_) if rest.iter().all(|r| matches!(r, WinningJudge(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let WinningJudge(d) = r {
                            serde_json::from_str(&d.encrypted_data)
                                .expect("Failed to deserialize WinningJudgeOutput")
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(CircuitEncryptedInputIdentifier::WinningJudge(items))
            }
            RoleAssignment(_) if rest.iter().all(|r| matches!(r, RoleAssignment(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let RoleAssignment(d) = r {
                            serde_json::from_str(&d.encrypted_data)
                                .expect("Failed to deserialize RoleAssignmentOutput")
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(CircuitEncryptedInputIdentifier::RoleAssignment(items))
            }
            KeyPublicize(_) if rest.iter().all(|r| matches!(r, KeyPublicize(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let KeyPublicize(d) = r {
                            serde_json::from_str(&d.encrypted_data)
                                .expect("Failed to deserialize KeyPublicizeOutput")
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(CircuitEncryptedInputIdentifier::KeyPublicize(items))
            }
            _ => Err("ClientRequestType variants are mixed; cannot convert".to_string()),
        },
        None => Err("Empty request list".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchStatus {
    Collecting,
    Processing,
    Completed,
    Failed,
}

impl Game {
    pub fn new(room_id: String, players: Vec<Player>) -> Self {
        Game {
            room_id: room_id.clone(),
            name: "".to_string(),
            players,
            max_players: 9,
            roles: vec![],
            phase: GamePhase::Waiting,
            day_count: 1,
            result: GameResult::InProgress,
            night_actions: NightActions::default(),
            vote_results: HashMap::new(),
            crypto_parameters: None,
            chat_log: super::chat::ChatLog::new(room_id),
            batch_request: BatchRequest::new(),
            computation_results: ComputationResults::default(),
            started_at: Some(Utc::now()),
        }
    }

    // 計算結果チェック用のメソッド
    pub fn has_role_assignment(&self) -> bool {
        self.computation_results.role_assignment.is_some()
    }

    pub fn has_divination_for_current_phase(&self) -> bool {
        // 現在のフェーズ・日数での占い結果が存在するかチェック
        self.computation_results
            .divination
            .iter()
            .any(|result| result.phase == self.phase && result.day_count == self.day_count)
    }

    // 特定のフェーズ・日数の占い結果を取得
    pub fn get_divination_for_phase(
        &self,
        phase: &GamePhase,
        day_count: u32,
    ) -> Option<&ComputationEntry<DivinationComputationResult>> {
        self.computation_results
            .divination
            .iter()
            .find(|result| result.phase == *phase && result.day_count == day_count)
    }

    // 占い結果を履歴に追加
    pub fn save_divination_result(
        &mut self,
        id: String,
        ciphertext: <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::Ciphertext,
    ) {
        self.computation_results.divination.push(ComputationEntry {
            id,
            computed_at: Utc::now(),
            phase: self.phase.clone(),
            day_count: self.day_count,
            result: DivinationComputationResult { ciphertext },
        });
    }

    // より厳密な占い可能性チェック
    pub fn can_perform_divination(&self) -> bool {
        self.phase == GamePhase::Night && !self.has_divination_for_current_phase()
    }

    // DivinationProcessing フェーズから Discussion フェーズへの遷移を管理
    pub async fn complete_divination_processing(&mut self, app_state: &crate::state::AppState) {
        if self.phase == GamePhase::DivinationProcessing {
            println!("Divination processing completed. Moving to Discussion phase.");

            self.change_phase(GamePhase::Discussion);

            // フェーズ変更をWebSocketで通知
            if let Err(e) = app_state
                .broadcast_phase_change(&self.room_id, "DivinationProcessing", "Discussion")
                .await
            {
                println!("Failed to broadcast phase change: {}", e);
            }
        }
    }

    // 次の日に進む（day_countを増やす）
    pub fn advance_to_next_day(&mut self) {
        self.day_count += 1;
        println!("Day count advanced to: {}", self.day_count);
    }

    // フェーズを変更し、必要に応じてday_countを増やす
    pub fn change_phase(&mut self, new_phase: GamePhase) {
        let old_phase = self.phase.clone();

        // Resultフェーズから次のNightフェーズに移る場合は新しい日
        if old_phase == GamePhase::Result && new_phase == GamePhase::Night {
            self.advance_to_next_day();
        }

        self.phase = new_phase.clone();
        self.add_phase_change_message(old_phase, new_phase);
    }
    pub fn save_role_assignment_result(
        &mut self,
        id: String,
        player_roles: Vec<PlayerRoleAssignment>,
        proof_data: serde_json::Value,
    ) {
        self.computation_results.role_assignment = Some(ComputationEntry {
            id,
            computed_at: Utc::now(),
            phase: self.phase.clone(),
            day_count: self.day_count,
            result: RoleAssignmentComputationResult {
                player_roles,
                proof_data,
            },
        });
    }

    // 夜アクション関連の実装
    pub fn register_attack(&mut self, target_id: &str) -> Result<(), String> {
        if !self.players.iter().any(|p| p.id.to_string() == target_id) {
            return Err("Target player not found".to_string());
        }
        self.night_actions.attacks.push(target_id.to_string());
        Ok(())
    }

    pub fn divine_player(&self, target_id: &str) -> Result<String, String> {
        let target = self
            .players
            .iter()
            .find(|p| p.id.to_string() == target_id)
            .ok_or("Target player not found")?;

        match &target.role {
            Some(role) => Ok(role.to_string()),
            None => Ok("Unknown".to_string()),
        }
    }

    pub fn register_guard(&mut self, target_id: &str) -> Result<(), String> {
        if !self.players.iter().any(|p| p.id.to_string() == target_id) {
            return Err("Target player not found".to_string());
        }
        self.night_actions.guards.push(target_id.to_string());
        Ok(())
    }

    pub fn resolve_night_actions(&mut self) {
        use std::collections::HashSet;
        let protected_players: HashSet<_> = self.night_actions.guards.iter().collect();

        for target_id in &self.night_actions.attacks {
            if !protected_players.contains(target_id) {
                if let Some(player) = self
                    .players
                    .iter_mut()
                    .find(|p| p.id.to_string() == *target_id)
                {
                    player.is_dead = true;
                }
            }
        }

        self.night_actions = NightActions::default();
    }

    // 投票システムの実装
    pub fn cast_vote(&mut self, voter_id: &str, target_id: &str) -> Result<(), String> {
        // プレイヤーの存在確認
        if !self.players.iter().any(|p| p.id == voter_id) {
            return Err("Voter not found".to_string());
        }
        if !self.players.iter().any(|p| p.id == target_id) {
            return Err("Vote target not found".to_string());
        }

        // 死亡プレイヤーのチェック
        if let Some(voter) = self.players.iter().find(|p| p.id == voter_id) {
            if voter.is_dead {
                return Err("Dead players cannot vote".to_string());
            }
        }

        // 二重投票チェック
        if self
            .vote_results
            .values()
            .any(|v| v.voters.contains(&voter_id.to_string()))
        {
            return Err("Already voted".to_string());
        }

        self.vote_results
            .entry(target_id.to_string())
            .or_insert_with(|| Vote {
                target_id: target_id.to_string(),
                voters: Vec::new(),
            })
            .voters
            .push(voter_id.to_string());

        Ok(())
    }

    pub fn count_votes(&self) -> Option<(String, usize)> {
        self.vote_results
            .iter()
            .max_by_key(|(_, vote)| vote.voters.len())
            .map(|(target_id, vote)| (target_id.clone(), vote.voters.len()))
    }

    pub fn resolve_voting(&mut self) {
        if let Some((target_id, _)) = self.count_votes() {
            if let Some(player) = self.players.iter_mut().find(|p| p.id == target_id) {
                player.is_dead = true;
            }
        }
        self.vote_results.clear();
    }
    pub fn add_phase_change_message(&mut self, from_phase: GamePhase, to_phase: GamePhase) {
        let message = match to_phase {
            GamePhase::Night => {
                "Night has fallen. Werewolves, choose your prey. Seer, choose your target."
            }
            GamePhase::DivinationProcessing => {
                "Processing divination results. Please wait a moment."
            }
            GamePhase::Discussion => "Morning has come. Let's discuss what happened last night.",
            GamePhase::Voting => {
                "It's time to vote. Cast your vote for the most suspicious person."
            }
            GamePhase::Result => "Voting has ended. Announcing results.",
            GamePhase::Finished => match self.result {
                GameResult::VillagerWin => "Villagers win!",
                GameResult::WerewolfWin => "Werewolves win!",
                GameResult::InProgress => "Game has ended.",
            },
            GamePhase::Waiting => "Waiting for game to start.",
        };

        self.chat_log.add_message(super::chat::ChatMessage::new(
            "system".to_string(),
            "System".to_string(),
            message.to_string(),
            super::chat::ChatMessageType::System,
        ));
    }

    pub async fn add_request(
        &mut self,
        request: ClientRequestType,
        app_state: &crate::state::AppState,
    ) -> String {
        // let mut batch_request = &self.batch_request;
        let size_limit = request.get_prover_count();

        if !self
            .batch_request
            .requests
            .iter()
            .any(|r| r.get_user_id() == request.get_user_id())
        {
            self.batch_request.requests.push(request);

            // バッチが満杯になったら処理を開始
            if self.batch_request.requests.len() >= size_limit {
                let batch_id = self.batch_request.batch_id.clone();

                // let mut service = self.clone();

                // TODO: 非同期でバッチ処理を開始
                // tokio::spawn(async move {
                // let mut games = game.lock().await;
                // if let Some(game) = games.get_mut(room_id) {
                //     // ゲームの状態を更新する処理
                //     // ...
                // }

                self.process_batch(app_state).await;
                // });

                // 新しいバッチを作成
                self.batch_request = BatchRequest::new();

                batch_id
            } else {
                self.batch_request.batch_id.clone()
            }
        } else {
            self.batch_request.batch_id.clone()
        }
    }

    async fn process_batch(&mut self, app_state: &crate::state::AppState) {
        self.batch_request.status = BatchStatus::Processing;

        // requsets: Vec<ClientRequestType>をCircuitEncryptedInputIdentifierに変換
        let identifier = try_convert_to_identifier(self.batch_request.requests.clone())
            .map_err(|e| {
                self.batch_request.status = BatchStatus::Failed;
                e
            })
            .unwrap();

        let client = Client::new();

        let mut responses = Vec::new();

        let req_to_node = ProofRequest {
            proof_id: self.batch_request.batch_id.clone(),
            circuit_type: identifier.clone(),
            output_type: ProofOutputType::Public,
        };

        // identifierをzk-mpc-nodeに送信するなどの処理を行う
        let node_urls = CONFIG.zk_mpc_node_urls();
        println!("Sending proof request to ZK-MPC nodes: {:?}", node_urls);

        for url in &node_urls {
            println!("Sending request to {}", url);
            let response = client
                .post(url)
                .json(&req_to_node)
                .send()
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to send request to {}: {}", url, e);
                    println!("ERROR: {}", error_msg);
                    error_msg
                });
            responses.push(response);
        }

        // レスポンスを処理
        for (url, response) in node_urls.iter().zip(responses) {
            match response {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(response_body) => {
                        println!("Response from {}: {:?}", url, response_body);
                    }
                    Err(e) => {
                        println!("ERROR: Failed to parse JSON response from {}: {}", url, e);
                        self.batch_request.status = BatchStatus::Failed;
                        self.chat_log.add_system_message(format!(
                            "Failed to process proof: Invalid response from ZK node at {}",
                            url
                        ));
                        return;
                    }
                },
                Err(e) => {
                    println!("ERROR: Failed to get response from {}: {}", url, e);
                    self.batch_request.status = BatchStatus::Failed;
                    self.chat_log.add_system_message(format!(
                        "Failed to connect to ZK node at {}: {}",
                        url, e
                    ));
                    return;
                }
            }
        }

        // 3. 結果の確認（非同期で実行）
        // tokio::spawn(async move {
        match check_status_with_retry(&self.batch_request.batch_id).await {
            Ok((true, Some(output))) => {
                println!(
                    "Proof ID {:?} is ready with output: {:?}",
                    self.batch_request.batch_id, output
                );
                // プルーフ生成成功時の処理
                // 例: WebSocketで結果をクライアントに通知
                match identifier {
                    CircuitEncryptedInputIdentifier::Divination(items) => {
                        // itemsを処理する
                        let divination_ciphertext = match output.value {
                            Some(bytes) => match CanonicalDeserialize::deserialize(&*bytes) {
                                Ok(result) => result,
                                Err(e) => {
                                    println!("Failed to deserialize divination result: {}", e);
                                    self.chat_log.add_system_message(
                                        "Failed to process divination result.".to_string(),
                                    );
                                    return;
                                }
                            },
                            None => {
                                println!("No output value found");
                                self.chat_log
                                    .add_system_message("Divination result not found.".to_string());
                                return;
                            }
                        };

                        // 占い結果を保存
                        self.save_divination_result(
                            self.batch_request.batch_id.clone(),
                            divination_ciphertext,
                        );
                        println!("Divination result processed successfully.");
                        self.chat_log.add_system_message(
                            "Divination result has been generated.".to_string(),
                        );

                        // 占い処理完了後、DivinationProcessingフェーズに移行
                        if self.phase == GamePhase::Night {
                            // self.change_phase(GamePhase::DivinationProcessing);

                            // フェーズ変更をWebSocketで通知
                            if let Err(e) = app_state
                                .broadcast_phase_change(
                                    &self.room_id,
                                    "Night",
                                    "DivinationProcessing",
                                )
                                .await
                            {
                                println!("Failed to broadcast phase change: {}", e);
                            }

                            // 3秒後にDiscussionフェーズに自動遷移
                            let room_id = self.room_id.clone();
                            let app_state_clone = app_state.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

                                // ゲーム状態を取得してDiscussionフェーズに遷移
                                let mut games = app_state_clone.games.lock().await;
                                if let Some(game) = games.get_mut(&room_id) {
                                    game.complete_divination_processing(&app_state_clone).await;
                                }
                            });
                        }

                        // 全プレイヤーに占い結果を通知
                        let latest_divination = self.computation_results.divination.last().unwrap();
                        let result_data = serde_json::json!({
                            "ciphertext": serde_json::to_value(&latest_divination.result.ciphertext).unwrap_or_default(),
                            "phase": latest_divination.phase,
                            "day_count": latest_divination.day_count,
                            "performed_at": latest_divination.computed_at,
                            "status": "ready"
                        });

                        if let Err(e) = app_state
                            .broadcast_computation_result(
                                &self.room_id,
                                "divination",
                                result_data,
                                None, // 全プレイヤーに送信
                                &self.batch_request.batch_id,
                            )
                            .await
                        {
                            println!("Failed to broadcast divination result: {}", e);
                        }
                    }
                    CircuitEncryptedInputIdentifier::AnonymousVoting(items) => {
                        println!("AnonymousVoting process is starting...");
                        // 1. outputのバイト列をFr型のtarget_idとして取得
                        let target_id: Fr = match output.value {
                            Some(bytes) => match CanonicalDeserialize::deserialize(&*bytes) {
                                Ok(id) => id,
                                Err(e) => {
                                    println!("Failed to deserialize target_id: {}", e);
                                    return;
                                }
                            },
                            None => {
                                println!("No output value found");
                                return;
                            }
                        };

                        println!("Deserialized target_id: {:?}", target_id);

                        // 2. Fr型のtarget_idをusize型のインデックスとして解釈
                        // Note: これはFr型の値がプレイヤーの配列のインデックスとして
                        // 適切な範囲内であることを前提としています
                        let target_index = {
                            // Fr型からBigUintに変換し、usizeに変換
                            let bytes = target_id.into_repr().to_bytes_le();
                            let index = bytes[0] as usize; // 最初のバイトをインデックスとして使用
                            if index >= self.players.len() {
                                println!("Invalid player index: {}", index);
                                return;
                            }
                            index
                        };

                        println!("Target index for voting: {}", target_index);

                        // 3. Kill the corresponding player
                        self.players[target_index].is_dead = true;

                        println!(
                            "Player {} has been killed.",
                            self.players[target_index].name
                        );

                        // 投票結果をログに追加
                        self.chat_log.add_system_message(format!(
                            "Voting result: {} has been executed.",
                            self.players[target_index].name
                        ));

                        println!(
                            "AnonymousVoting processed successfully for target_id: {}",
                            target_id
                        );

                        // Update phase
                        self.change_phase(GamePhase::Result);

                        // 4. Clear vote results
                        self.vote_results.clear();

                        println!("Vote results cleared after processing.");

                        // 全プレイヤーに投票結果を通知
                        let result_data = serde_json::json!({
                            "executed_player_id": target_id.into_repr().to_string(),
                            "executed_player_name": self.players[target_index].name,
                            "status": "completed"
                        });

                        if let Err(e) = app_state
                            .broadcast_computation_result(
                                &self.room_id,
                                "anonymous_voting",
                                result_data,
                                None, // 全プレイヤーに送信
                                &self.batch_request.batch_id,
                            )
                            .await
                        {
                            println!("Failed to broadcast voting result: {}", e);
                        }
                    }
                    CircuitEncryptedInputIdentifier::WinningJudge(items) => {
                        // itemsを処理する

                        let game_state: Fr = match output.value {
                            Some(bytes) => match CanonicalDeserialize::deserialize(&*bytes) {
                                Ok(state) => state,
                                Err(e) => {
                                    println!("Failed to deserialize game_state: {}", e);
                                    return;
                                }
                            },
                            None => {
                                println!("No output value found");
                                return;
                            }
                        };

                        // 状態をゲームの結果に反映

                        let result = if game_state == Fr::from(1u32) {
                            GameResult::WerewolfWin
                        } else if game_state == Fr::from(2u32) {
                            GameResult::VillagerWin
                        } else {
                            self.chat_log
                                .add_system_message("The game continues.".to_string());
                            GameResult::InProgress
                        };

                        if result != GameResult::InProgress {
                            let winner_message = match result {
                                GameResult::VillagerWin => "Villagers win!",
                                GameResult::WerewolfWin => "Werewolves win!",
                                GameResult::InProgress => unreachable!(),
                            };

                            self.chat_log.add_message(ChatMessage::new(
                                "system".to_string(),
                                "System".to_string(),
                                format!("{}", winner_message),
                                ChatMessageType::System,
                            ));

                            self.change_phase(GamePhase::Finished);
                        }

                        self.result = result.clone();

                        // 全プレイヤーに勝利判定結果を通知
                        let alive_players: Vec<String> = self
                            .players
                            .iter()
                            .filter(|p| !p.is_dead)
                            .map(|p| p.id.clone())
                            .collect();

                        let result_data = serde_json::json!({
                            "game_result": result,
                            "alive_players": alive_players,
                            "game_state_value": game_state.into_repr().to_string(),
                            "status": "completed"
                        });

                        if let Err(e) = app_state
                            .broadcast_computation_result(
                                &self.room_id,
                                "winning_judge",
                                result_data,
                                None, // 全プレイヤーに送信
                                &self.batch_request.batch_id,
                            )
                            .await
                        {
                            println!("Failed to broadcast winning judge result: {}", e);
                        }
                    }
                    CircuitEncryptedInputIdentifier::RoleAssignment(items) => {
                        // itemsを処理する
                        let role_result: Vec<Fr> = match output.value {
                            Some(bytes) => match CanonicalDeserialize::deserialize(&*bytes) {
                                Ok(state) => state,
                                Err(e) => {
                                    println!("Failed to deserialize role_result: {}", e);
                                    return;
                                }
                            },
                            None => {
                                println!("No output value found");
                                return;
                            }
                        };

                        // role_resultをゲームの結果に反映
                        let mut player_role_assignments = Vec::new();
                        for (player, role) in self.players.iter_mut().zip(role_result.iter()) {
                            let assigned_role = if *role == Fr::from(0u32) {
                                Some(Role::Villager)
                            } else if *role == Fr::from(1u32) {
                                Some(Role::Seer)
                            } else if *role == Fr::from(2u32) {
                                Some(Role::Werewolf)
                            } else {
                                None
                            };

                            player.role = assigned_role.clone();

                            if let Some(role) = assigned_role {
                                player_role_assignments.push(PlayerRoleAssignment {
                                    player_id: player.id.clone(),
                                    role: format!("{:?}", role),
                                });
                            }
                        }

                        // 計算結果を保存
                        self.save_role_assignment_result(
                            self.batch_request.batch_id.clone(),
                            player_role_assignments.clone(),
                            serde_json::json!({
                                "raw_role_result": role_result.iter().map(|r| r.into_repr().to_string()).collect::<Vec<_>>(),
                                "computation_time": chrono::Utc::now().to_rfc3339()
                            })
                        );

                        self.chat_log.add_system_message(format!(
                            "Roles have been assigned: {}. Starting the game.",
                            self.players
                                .iter()
                                .map(|p| format!("{}: {:?}", p.name, p.role.as_ref().unwrap()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));

                        // 全プレイヤーに役職配布結果を通知
                        let role_assignments: Vec<_> = self
                            .players
                            .iter()
                            .map(|p| {
                                serde_json::json!({
                                    "player_id": p.id,
                                    "player_name": p.name,
                                    "role": p.role
                                })
                            })
                            .collect();

                        let result_data = serde_json::json!({
                            "role_assignments": role_assignments,
                            "status": "completed"
                        });

                        if let Err(e) = app_state
                            .broadcast_computation_result(
                                &self.room_id,
                                "role_assignment",
                                result_data,
                                None, // 全プレイヤーに送信
                                &self.batch_request.batch_id,
                            )
                            .await
                        {
                            println!("Failed to broadcast role assignment result: {}", e);
                        }
                    }
                    CircuitEncryptedInputIdentifier::KeyPublicize(items) => {
                        // itemsを処理する
                        let public_key: <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::PublicKey =
                            match output.value {
                                Some(bytes) => match CanonicalDeserialize::deserialize(&*bytes) {
                                    Ok(key) => key,
                                    Err(e) => {
                                        println!("Failed to deserialize public key: {}", e);
                                        return;
                                    }
                                },
                                None => {
                                    println!("No output value found");
                                    return;
                                }
                            };

                        self.crypto_parameters = Some(CryptoParameters {
                            fortune_teller_public_key: public_key,
                            ..self.crypto_parameters.clone().unwrap()
                        });
                    }
                }

                self.batch_request.status = BatchStatus::Completed;
            }
            _ => {
                // 失敗時の処理
                println!("Proof ID {:?} is failed", self.batch_request.batch_id);
                self.batch_request.status = BatchStatus::Failed;
            }
        }
        // });
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Game {{ room_id: {}, name: {}, players: {:?}, phase: {:?}, result: {:?} }}",
            self.room_id, self.name, self.players, self.phase, self.result
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct CryptoParameters {
    // public
    pub pedersen_param: <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::Parameters,
    pub player_commitment:
        Vec<<<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::Output>,
    pub fortune_teller_public_key:
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::PublicKey,
    pub elgamal_param:
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::Parameters,

    // TODO: do not put the secret key in the struct
    // secret
    pub player_randomness: Vec<Fr>,
    pub secret_key:
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::SecretKey,
}

impl Clone for CryptoParameters {
    fn clone(&self) -> Self {
        // Dummy implementation
        Self {
            pedersen_param: self.pedersen_param.clone(),
            player_commitment: self.player_commitment.clone(),
            fortune_teller_public_key: self.fortune_teller_public_key,
            elgamal_param: self.elgamal_param.clone(),
            player_randomness: self.player_randomness.clone(),
            secret_key: ark_crypto_primitives::encryption::elgamal::SecretKey(
                self.secret_key.0.clone(),
            ),
        }
    }
}

impl std::fmt::Debug for CryptoParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!("Debug not implemented for CryptoParameters");
    }
}
