use crate::{
    blockchain::{
        state_hash::{compute_game_id, compute_proof_id},
        ProofType as ChainProofType,
    },
    models::{
        chat::{ChatMessage, ChatMessageType},
        role::Role,
    },
};

use super::player::Player;
use ark_bn254::Fr;
use ark_crypto_primitives::{encryption::AsymmetricEncryptionScheme, CommitmentScheme};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalDeserialize;
use base64;
use chrono::{DateTime, Utc};
use derivative::Derivative;
use mpc_algebra_wasm::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Derivative, Clone)]
#[derivative(Debug)]
pub struct Game {
    pub room_id: String,
    pub name: String,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub phase: GamePhase,
    pub day_count: u32,
    pub result: GameResult,
    pub night_actions: NightActions,
    pub vote_results: HashMap<String, Vote>,
    pub crypto_parameters: Option<CryptoParameters>,
    pub chat_log: super::chat::ChatLog,
    #[derivative(Debug = "ignore")]
    pub batch_request: BatchRequest,
    #[derivative(Debug = "ignore")]
    #[serde(skip)]
    pub active_batches: HashMap<BatchKey, BatchRequest>,
    pub computation_results: ComputationResults,
    pub started_at: Option<DateTime<Utc>>,
    pub phase_started_at: DateTime<Utc>,
    pub grouping_parameter: GroupingParameter,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NightActions {
    pub attacks: Vec<String>, // 襲撃対象
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
    #[serde(default)]
    pub is_dummy: bool,
    #[serde(default)]
    pub public_key: Option<String>, // Curve25519公開鍵（Base64エンコード）
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
    pub(crate) fn get_prover_count(&self) -> usize {
        match self {
            ClientRequestType::Divination(info)
            | ClientRequestType::AnonymousVoting(info)
            | ClientRequestType::WinningJudge(info)
            | ClientRequestType::RoleAssignment(info)
            | ClientRequestType::KeyPublicize(info) => info.prover_count,
        }
    }

    pub(crate) fn get_user_id(&self) -> &str {
        match self {
            ClientRequestType::Divination(info)
            | ClientRequestType::AnonymousVoting(info)
            | ClientRequestType::WinningJudge(info)
            | ClientRequestType::RoleAssignment(info)
            | ClientRequestType::KeyPublicize(info) => info.user_id.as_str(),
        }
    }

    pub(crate) fn get_public_key(&self) -> Option<&str> {
        match self {
            ClientRequestType::Divination(info)
            | ClientRequestType::AnonymousVoting(info)
            | ClientRequestType::WinningJudge(info)
            | ClientRequestType::RoleAssignment(info)
            | ClientRequestType::KeyPublicize(info) => info.public_key.as_deref(),
        }
    }

    pub(crate) fn is_dummy(&self) -> bool {
        match self {
            ClientRequestType::Divination(info) => info.is_dummy,
            _ => false,
        }
    }

    pub fn get_proof_type(&self) -> ProofTypeKey {
        match self {
            ClientRequestType::Divination(_) => ProofTypeKey::Divination,
            ClientRequestType::AnonymousVoting(_) => ProofTypeKey::AnonymousVoting,
            ClientRequestType::WinningJudge(_) => ProofTypeKey::WinningJudge,
            ClientRequestType::RoleAssignment(_) => ProofTypeKey::RoleAssignment,
            ClientRequestType::KeyPublicize(_) => ProofTypeKey::KeyPublicize,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProofTypeKey {
    RoleAssignment,
    Divination,
    AnonymousVoting,
    WinningJudge,
    KeyPublicize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CircuitProfileKey {
    pub player_count: usize,
    pub werewolf_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BatchKey {
    pub room_id: String,
    pub phase: GamePhase,
    pub day_count: u32,
    pub proof_type: ProofTypeKey,
    pub circuit_profile: CircuitProfileKey,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub batch_id: String,
    pub requests: Vec<ClientRequestType>,
    #[serde(default)]
    pub expected_prover_count: usize,
    pub status: BatchStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl BatchRequest {
    pub fn new(expected_prover_count: usize) -> Self {
        BatchRequest {
            batch_id: uuid::Uuid::new_v4().to_string(),
            requests: Vec::new(),
            expected_prover_count,
            status: BatchStatus::Collecting,
            created_at: chrono::Utc::now(),
        }
    }
}

pub(crate) fn try_convert_to_identifier(
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
                            serde_json::from_str(&d.encrypted_data).map_err(|e| {
                                format!("Failed to deserialize DivinationOutput: {}", e)
                            })
                        } else {
                            Err("Unexpected request type in Divination batch".to_string())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CircuitEncryptedInputIdentifier::Divination(items))
            }
            AnonymousVoting(_) if rest.iter().all(|r| matches!(r, AnonymousVoting(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let AnonymousVoting(d) = r {
                            serde_json::from_str(&d.encrypted_data).map_err(|e| {
                                format!("Failed to deserialize AnonymousVotingOutput: {}", e)
                            })
                        } else {
                            Err("Unexpected request type in AnonymousVoting batch".to_string())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CircuitEncryptedInputIdentifier::AnonymousVoting(items))
            }
            WinningJudge(_) if rest.iter().all(|r| matches!(r, WinningJudge(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let WinningJudge(d) = r {
                            serde_json::from_str(&d.encrypted_data).map_err(|e| {
                                format!("Failed to deserialize WinningJudgeOutput: {}", e)
                            })
                        } else {
                            Err("Unexpected request type in WinningJudge batch".to_string())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CircuitEncryptedInputIdentifier::WinningJudge(items))
            }
            RoleAssignment(_) if rest.iter().all(|r| matches!(r, RoleAssignment(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let RoleAssignment(d) = r {
                            serde_json::from_str(&d.encrypted_data).map_err(|e| {
                                format!("Failed to deserialize RoleAssignmentOutput: {}", e)
                            })
                        } else {
                            Err("Unexpected request type in RoleAssignment batch".to_string())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CircuitEncryptedInputIdentifier::RoleAssignment(items))
            }
            KeyPublicize(_) if rest.iter().all(|r| matches!(r, KeyPublicize(_))) => {
                let items = requests
                    .into_iter()
                    .map(|r| {
                        if let KeyPublicize(d) = r {
                            serde_json::from_str(&d.encrypted_data).map_err(|e| {
                                format!("Failed to deserialize KeyPublicizeOutput: {}", e)
                            })
                        } else {
                            Err("Unexpected request type in KeyPublicize batch".to_string())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
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

pub struct BatchEnqueueResult {
    pub batch_id: String,
    pub should_process: bool,
}

pub enum BatchEnqueueError {
    Conflict(String),
}

impl Game {
    pub fn new(
        room_id: String,
        players: Vec<Player>,
        max_players: usize,
        grouping_parameter: GroupingParameter,
    ) -> Self {
        Game {
            room_id: room_id.clone(),
            name: "".to_string(),
            players,
            max_players,
            phase: GamePhase::Waiting,
            day_count: 1,
            result: GameResult::InProgress,
            night_actions: NightActions::default(),
            vote_results: HashMap::new(),
            crypto_parameters: None,
            chat_log: super::chat::ChatLog::new(room_id),
            batch_request: BatchRequest::new(0),
            active_batches: HashMap::new(),
            computation_results: ComputationResults::default(),
            started_at: Some(Utc::now()),
            phase_started_at: Utc::now(),
            grouping_parameter,
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

    pub fn has_pending_or_processing_batches(&self) -> bool {
        self.batch_request.status == BatchStatus::Processing
            || !self.batch_request.requests.is_empty()
            || !self.active_batches.is_empty()
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
        self.phase_started_at = Utc::now();
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

    // pub fn divine_player(&self, target_id: &str) -> Result<String, String> {
    //     let target = self
    //         .players
    //         .iter()
    //         .find(|p| p.id.to_string() == target_id)
    //         .ok_or("Target player not found")?;

    //     match &target.role {
    //         Some(role) => Ok(role.to_string()),
    //         None => Ok("Unknown".to_string()),
    //     }
    // }

    pub fn resolve_night_actions(&mut self) {
        for target_id in &self.night_actions.attacks {
            if let Some(player) = self
                .players
                .iter_mut()
                .find(|p| p.id.to_string() == *target_id)
            {
                player.is_dead = true;
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
    pub fn add_phase_change_message(&mut self, _from_phase: GamePhase, to_phase: GamePhase) {
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

    pub fn build_batch_key(&self, request: &ClientRequestType) -> BatchKey {
        BatchKey {
            room_id: self.room_id.clone(),
            phase: self.phase.clone(),
            day_count: self.day_count,
            proof_type: request.get_proof_type(),
            circuit_profile: CircuitProfileKey {
                player_count: self.grouping_parameter.get_num_players(),
                werewolf_count: self.grouping_parameter.get_werewolf_count(),
            },
        }
    }

    pub fn add_request_to_batch(
        &mut self,
        request: ClientRequestType,
    ) -> Result<BatchEnqueueResult, BatchEnqueueError> {
        let expected_prover_count = request.get_prover_count();
        if expected_prover_count == 0 {
            return Err(BatchEnqueueError::Conflict(
                "prover_count must be greater than zero".to_string(),
            ));
        }

        let batch_key = self.build_batch_key(&request);
        let batch = self
            .active_batches
            .entry(batch_key.clone())
            .or_insert_with(|| BatchRequest::new(expected_prover_count));

        if batch.expected_prover_count != expected_prover_count {
            return Err(BatchEnqueueError::Conflict(
                "conflicting prover_count for the same batch key".to_string(),
            ));
        }

        if batch
            .requests
            .iter()
            .any(|r| r.get_proof_type() != request.get_proof_type())
        {
            return Err(BatchEnqueueError::Conflict(
                "proof type mismatch in existing batch".to_string(),
            ));
        }

        if let Some(existing_index) = batch
            .requests
            .iter()
            .position(|r| r.get_user_id() == request.get_user_id())
        {
            let should_replace = batch.requests[existing_index].is_dummy() && !request.is_dummy();
            if should_replace {
                batch.requests[existing_index] = request;
            }
        } else {
            batch.requests.push(request);
        }

        let batch_id = batch.batch_id.clone();
        let should_process = batch.requests.len() >= batch.expected_prover_count;

        if should_process {
            if let Some(mut ready_batch) = self.active_batches.remove(&batch_key) {
                ready_batch.status = BatchStatus::Collecting;
                self.batch_request = ready_batch;
            }
        }

        Ok(BatchEnqueueResult {
            batch_id,
            should_process,
        })
    }

    pub async fn add_request(
        &mut self,
        request: ClientRequestType,
        app_state: &crate::state::AppState,
    ) -> String {
        match self.add_request_to_batch(request) {
            Ok(result) => {
                if result.should_process {
                    self.process_current_batch(app_state).await;
                }
                result.batch_id
            }
            Err(BatchEnqueueError::Conflict(_)) => self.batch_request.batch_id.clone(),
        }
    }

    pub async fn process_current_batch(&mut self, app_state: &crate::state::AppState) {
        if self.batch_request.requests.is_empty() {
            return;
        }

        self.process_batch(app_state).await;
        self.batch_request = BatchRequest::new(0);
    }

    async fn process_batch(&mut self, app_state: &crate::state::AppState) {
        self.batch_request.status = BatchStatus::Processing;

        let proof_execution = match crate::services::zk_proof::take_precomputed_batch_result(
            &self.batch_request.batch_id,
        )
        .await
        {
            Some(result) => result,
            None => crate::services::zk_proof::execute_batch_request(&self.batch_request).await,
        };

        match proof_execution {
            Ok((identifier, output)) => {
                println!(
                    "Proof ID {:?} is ready with output: {:?}",
                    self.batch_request.batch_id, output
                );

                if app_state.blockchain_client.is_enabled() {
                    let proof_id = compute_proof_id(&self.batch_request.batch_id);
                    let game_id = compute_game_id(&self.room_id);
                    let Some(circuit_profile) = identifier.circuit_profile() else {
                        self.batch_request.status = BatchStatus::Failed;
                        self.chat_log.add_system_message(
                            "On-chain proof verification skipped: failed to derive circuit profile."
                                .to_string(),
                        );
                        return;
                    };
                    if !circuit_profile.is_supported_onchain_profile() {
                        self.batch_request.status = BatchStatus::Failed;
                        self.chat_log.add_system_message(
                            "On-chain proof verification skipped: unsupported circuit profile."
                                .to_string(),
                        );
                        return;
                    }
                    let (proof_type, proof_type_label) = match &identifier {
                        CircuitEncryptedInputIdentifier::RoleAssignment(_) => {
                            (ChainProofType::RoleAssignment, "RoleAssignment")
                        }
                        CircuitEncryptedInputIdentifier::Divination(_) => {
                            (ChainProofType::Divination, "Divination")
                        }
                        CircuitEncryptedInputIdentifier::AnonymousVoting(_) => {
                            (ChainProofType::AnonymousVoting, "AnonymousVoting")
                        }
                        CircuitEncryptedInputIdentifier::WinningJudge(_) => {
                            (ChainProofType::WinningJudgement, "WinningJudgement")
                        }
                        CircuitEncryptedInputIdentifier::KeyPublicize(_) => {
                            (ChainProofType::KeyPublicize, "KeyPublicize")
                        }
                    };
                    let player_count: u8 = match u8::try_from(circuit_profile.player_count()) {
                        Ok(v) => v,
                        Err(_) => {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(format!(
                                "On-chain proof verification skipped: invalid player count for {}.",
                                proof_type_label
                            ));
                            return;
                        }
                    };
                    let werewolf_count: u8 = match u8::try_from(circuit_profile.werewolf_count()) {
                        Ok(v) => v,
                        Err(_) => {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(format!(
                                "On-chain proof verification skipped: invalid werewolf count for {}.",
                                proof_type_label
                            ));
                            return;
                        }
                    };
                    let proof_data = match output.proof.clone() {
                        Some(bytes) if !bytes.is_empty() => bytes,
                        _ => {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(format!(
                                "On-chain proof verification skipped: missing proof bytes for {}.",
                                proof_type_label
                            ));
                            return;
                        }
                    };
                    let public_inputs = match output.public_inputs.clone() {
                        Some(bytes) => bytes,
                        None if matches!(proof_type, ChainProofType::KeyPublicize) => Vec::new(),
                        None => {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(format!(
                                "On-chain proof verification skipped: missing public inputs for {}.",
                                proof_type_label
                            ));
                            return;
                        }
                    };

                    match app_state
                        .blockchain_client
                        .verify_proof(
                            proof_id,
                            game_id,
                            proof_type,
                            player_count,
                            werewolf_count,
                            &proof_data,
                            &public_inputs,
                        )
                        .await
                    {
                        Ok(Some(true)) => {
                            self.chat_log.add_system_message(format!(
                                "On-chain proof verification succeeded: {}.",
                                proof_type_label
                            ));
                        }
                        Ok(None) => {}
                        Ok(Some(false)) => {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(
                                "On-chain proof verification failed.".to_string(),
                            );
                            return;
                        }
                        Err(e) => {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(format!(
                                "On-chain proof verification error: {}",
                                e
                            ));
                            return;
                        }
                    }
                }

                // プルーフ生成成功時の処理
                // 例: WebSocketで結果をクライアントに通知
                match identifier {
                    CircuitEncryptedInputIdentifier::Divination(_items) => {
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
                        let Some(latest_divination) = self.computation_results.divination.last()
                        else {
                            self.batch_request.status = BatchStatus::Failed;
                            self.chat_log.add_system_message(
                                "Divination result was not persisted correctly.".to_string(),
                            );
                            return;
                        };
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
                    CircuitEncryptedInputIdentifier::AnonymousVoting(_items) => {
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

                        // フェーズ変更をWebSocketで通知
                        if let Err(e) = app_state
                            .broadcast_phase_change(&self.room_id, "Voting", "Result")
                            .await
                        {
                            println!("Failed to broadcast phase change: {}", e);
                        }

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
                    CircuitEncryptedInputIdentifier::WinningJudge(_items) => {
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
                        if result != GameResult::InProgress {
                            let mut rooms = app_state.rooms.lock().await;
                            if let Some(room) = rooms.get_mut(&self.room_id) {
                                room.status = crate::models::room::RoomStatus::Closed;
                            }
                        }

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
                    CircuitEncryptedInputIdentifier::RoleAssignment(_items) => {
                        println!("RoleAssignment process is starting...");

                        // ProofOutputからEncryptedSharesを取得
                        if let Some(encrypted_shares) = output.shares {
                            println!("Received {} encrypted role shares", encrypted_shares.len());

                            // 各プレイヤーに個別に暗号化された役職データを配信
                            for encrypted_share in encrypted_shares {
                                let user_index_str = encrypted_share.user_id.clone();
                                let node_id = encrypted_share.node_id;

                                // user_idは配列のインデックス（文字列）なので、実際のプレイヤーIDに変換
                                let user_index: usize = match user_index_str.parse() {
                                    Ok(idx) => idx,
                                    Err(e) => {
                                        println!(
                                            "ERROR: Failed to parse user_id '{}' as index: {}",
                                            user_index_str, e
                                        );
                                        continue;
                                    }
                                };

                                // インデックスからプレイヤーIDを取得
                                let actual_player_id = if user_index < self.players.len() {
                                    self.players[user_index].id.clone()
                                } else {
                                    println!(
                                        "ERROR: Player index {} out of bounds (total players: {})",
                                        user_index,
                                        self.players.len()
                                    );
                                    continue;
                                };

                                println!(
                                    "Converting user_index {} to player_id {}",
                                    user_index, actual_player_id
                                );

                                // encrypted_dataは [nonce(24バイト) + ciphertext] の形式
                                if encrypted_share.encrypted_data.len() < 24 {
                                    println!(
                                        "ERROR: Encrypted data too short for player {} (index {})",
                                        actual_player_id, user_index
                                    );
                                    continue;
                                }

                                // nonceとciphertextを分離
                                let nonce = &encrypted_share.encrypted_data[..24];
                                let ciphertext = &encrypted_share.encrypted_data[24..];

                                // Base64エンコード
                                let nonce_b64 = base64::encode(nonce);
                                let ciphertext_b64 = base64::encode(ciphertext);

                                // サーバーは中継のみ - node_idを送信してクライアント側で公開鍵を解決
                                let result_data = serde_json::json!({
                                    "encrypted_role": {
                                        "encrypted": ciphertext_b64,
                                        "nonce": nonce_b64,
                                        "node_id": node_id
                                    },
                                    "status": "ready"
                                });

                                println!(
                                    "Sending encrypted role to player {} (index {}, from node {})",
                                    actual_player_id, user_index, node_id
                                );

                                // 特定のプレイヤーにのみ送信（実際のプレイヤーIDを使用）
                                if let Err(e) = app_state
                                    .broadcast_computation_result(
                                        &self.room_id,
                                        "role_assignment",
                                        result_data,
                                        Some(actual_player_id.clone()), // 実際のプレイヤーIDを使用
                                        &self.batch_request.batch_id,
                                    )
                                    .await
                                {
                                    println!(
                                        "Failed to send encrypted role to player {}: {}",
                                        actual_player_id, e
                                    );
                                }
                            }

                            self.chat_log.add_system_message(
                                "Roles have been assigned. Check your private information."
                                    .to_string(),
                            );
                        } else {
                            // 後方互換性のため、古い実装も残す（デバッグモード用）
                            println!("No encrypted shares found, using fallback role assignment");

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

                            let mut player_role: Option<Role> = None;

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

                                player_role = assigned_role.clone();

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

                            // self.chat_log.add_system_message(format!(
                            //     "Roles have been assigned: {}. Starting the game.",
                            //     self.players
                            //         .iter()
                            //         .map(|p| format!("{}: {:?}", p.name, p.role.as_ref().unwrap()))
                            //         .collect::<Vec<_>>()
                            //         .join(", ")
                            // ));

                            // 全プレイヤーに役職配布結果を通知
                            let role_assignments: Vec<_> = self
                                .players
                                .iter()
                                .map(|p| {
                                    serde_json::json!({
                                        "player_id": p.id,
                                        "player_name": p.name,
                                        "role": player_role
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
                    }
                    CircuitEncryptedInputIdentifier::KeyPublicize(_items) => {
                        println!("KeyPublicize process is starting...");

                        // ProofOutputから公開鍵を取得
                        let (pub_key_x, pub_key_y): (Fr, Fr) = match output.value {
                            Some(bytes) => match CanonicalDeserialize::deserialize(&*bytes) {
                                Ok(key) => key,
                                Err(e) => {
                                    println!("Failed to deserialize divination public key: {}", e);
                                    return;
                                }
                            },
                            None => {
                                println!("No divination public key found in output");
                                return;
                            }
                        };

                        let pub_key_x_json = serde_json::to_string(&pub_key_x)
                            .unwrap_or_else(|_| "\"serialization_error\"".to_string());
                        let pub_key_y_json = serde_json::to_string(&pub_key_y)
                            .unwrap_or_else(|_| "\"serialization_error\"".to_string());

                        println!("Fortune teller public key received from KeyPublicize MPC:");
                        println!("  X: {}", pub_key_x_json);
                        println!("  Y: {}", pub_key_y_json);

                        // EdwardsProjectiveに変換してcrypto_parametersに保存

                        let mut curve_pt = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPubKey::default();

                        curve_pt.x = pub_key_x;
                        curve_pt.y = pub_key_y;

                        // crypto_parametersのfortune_teller_public_keyを更新
                        if let Some(ref mut crypto_params) = self.crypto_parameters {
                            crypto_params.fortune_teller_public_key = Some(curve_pt);
                            println!("Updated crypto_parameters.fortune_teller_public_key with KeyPublicize result");
                        } else {
                            println!("WARNING: crypto_parameters is None, cannot update fortune_teller_public_key");
                        }

                        // 全プレイヤーに占い公開鍵を配信（フロントエンド互換形式）
                        let key_data = serde_json::json!({
                            "divination_public_key": {
                                "x": pub_key_x_json,
                                "y": pub_key_y_json,
                                "_params": null
                            },
                            "status": "completed"
                        });

                        if let Err(e) = app_state
                            .broadcast_computation_result(
                                &self.room_id,
                                "divination_key_ready",
                                key_data,
                                None, // 全プレイヤーに送信
                                &self.batch_request.batch_id,
                            )
                            .await
                        {
                            println!("Failed to broadcast divination key ready event: {}", e);
                        }

                        self.chat_log.add_system_message(
                            "Fortune teller public key has been generated via MPC.".to_string(),
                        );
                    }
                }

                self.batch_request.status = BatchStatus::Completed;
            }
            Err(error) => {
                // 失敗時の処理
                println!(
                    "Proof ID {:?} is failed: {}",
                    self.batch_request.batch_id, error
                );
                self.batch_request.status = BatchStatus::Failed;
                self.chat_log
                    .add_system_message(format!("Failed to process proof: {}", error));
            }
        }
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
    // public parameters only - secret information is NOT stored here
    pub pedersen_param: <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::Parameters,
    pub player_commitment:
        Vec<<<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::Output>,
    pub fortune_teller_public_key: Option<
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::PublicKey,
    >,
    pub elgamal_param:
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::Parameters,
}

impl Clone for CryptoParameters {
    fn clone(&self) -> Self {
        Self {
            pedersen_param: self.pedersen_param.clone(),
            player_commitment: self.player_commitment.clone(),
            fortune_teller_public_key: self.fortune_teller_public_key,
            elgamal_param: self.elgamal_param.clone(),
        }
    }
}

impl std::fmt::Debug for CryptoParameters {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!("Debug not implemented for CryptoParameters");
    }
}
