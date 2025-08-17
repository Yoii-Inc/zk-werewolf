use crate::services::zk_proof::check_status_with_retry;

use super::player::Player;
use ark_bls12_377::Fr;
use ark_crypto_primitives::{encryption::AsymmetricEncryptionScheme, CommitmentScheme};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalDeserialize;
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
    pub result: GameResult,
    pub night_actions: NightActions,
    pub vote_results: HashMap<String, Vote>,
    pub crypto_parameters: Option<CryptoParameters>,
    pub chat_log: super::chat::ChatLog,
    #[derivative(Debug = "ignore")]
    pub batch_request: BatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GamePhase {
    Waiting,    // ゲーム開始前
    Night,      // 夜フェーズ
    Discussion, // 議論フェーズ
    Voting,     // 投票フェーズ
    Result,     // 結果発表フェーズ
    Finished,   // ゲーム終了
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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub batch_id: String,
    pub requests: Vec<ClientRequestType>,
    pub status: BatchStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

const ZK_MPC_NODE_URL: [&str; 3] = [
    "http://localhost:9000",
    "http://localhost:9001",
    "http://localhost:9002",
];

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
            result: GameResult::InProgress,
            night_actions: NightActions::default(),
            vote_results: HashMap::new(),
            crypto_parameters: None,
            chat_log: super::chat::ChatLog::new(room_id),
            batch_request: BatchRequest::new(),
        }
    }

    // 夜アクション関連の実装
    pub fn register_attack(&mut self, target_id: &str) -> Result<(), String> {
        if !self.players.iter().any(|p| p.id.to_string() == target_id) {
            return Err("対象プレイヤーが見つかりません".to_string());
        }
        self.night_actions.attacks.push(target_id.to_string());
        Ok(())
    }

    pub fn divine_player(&self, target_id: &str) -> Result<String, String> {
        let target = self
            .players
            .iter()
            .find(|p| p.id.to_string() == target_id)
            .ok_or("対象プレイヤーが見つかりません")?;

        match &target.role {
            Some(role) => Ok(role.to_string()),
            None => Ok("不明".to_string()),
        }
    }

    pub fn register_guard(&mut self, target_id: &str) -> Result<(), String> {
        if !self.players.iter().any(|p| p.id.to_string() == target_id) {
            return Err("対象プレイヤーが見つかりません".to_string());
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
            return Err("投票者が見つかりません".to_string());
        }
        if !self.players.iter().any(|p| p.id == target_id) {
            return Err("投票対象が見つかりません".to_string());
        }

        // 死亡プレイヤーのチェック
        if let Some(voter) = self.players.iter().find(|p| p.id == voter_id) {
            if voter.is_dead {
                return Err("死亡したプレイヤーは投票できません".to_string());
            }
        }

        // 二重投票チェック
        if self
            .vote_results
            .values()
            .any(|v| v.voters.contains(&voter_id.to_string()))
        {
            return Err("既に投票済みです".to_string());
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
                "夜になりました。人狼は獲物を選び、占い師は占う相手を選んでください。"
            }
            GamePhase::Discussion => "朝になりました。昨晩の出来事について話し合いましょう。",
            GamePhase::Voting => "投票の時間です。最も疑わしい人物に投票してください。",
            GamePhase::Result => "投票が終了しました。結果を発表します。",
            GamePhase::Finished => match self.result {
                GameResult::VillagerWin => "村人陣営の勝利です！",
                GameResult::WerewolfWin => "人狼陣営の勝利です！",
                GameResult::InProgress => "ゲームが終了しました。",
            },
            GamePhase::Waiting => "ゲームの開始を待っています。",
        };

        self.chat_log.add_message(super::chat::ChatMessage::new(
            "system".to_string(),
            "システム".to_string(),
            message.to_string(),
            super::chat::ChatMessageType::System,
        ));
    }

    pub async fn add_request(&mut self, request: ClientRequestType) -> String {
        // let mut batch_request = &self.batch_request;
        let size_limit = request.get_prover_count();
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

            self.process_batch().await;
            // });

            // 新しいバッチを作成
            self.batch_request = BatchRequest::new();

            batch_id
        } else {
            self.batch_request.batch_id.clone()
        }
    }

    async fn process_batch(&mut self) {
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
        for port in ZK_MPC_NODE_URL {
            let response = client
                .post(port)
                .json(&req_to_node)
                .send()
                .await
                .map_err(|e| e.to_string());
            responses.push(response);
        }

        for (port, response) in ZK_MPC_NODE_URL.iter().zip(responses) {
            let response_body: serde_json::Value = response.unwrap().json().await.unwrap();
            println!("Response from port {}: {:?}", port, response_body);
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

                        // 3. 該当するプレイヤーを死亡させる
                        self.players[target_index].is_dead = true;

                        println!(
                            "Player {} has been killed.",
                            self.players[target_index].name
                        );

                        // 5. 投票結果をログに追加
                        self.chat_log.add_system_message(format!(
                            "投票結果: {}が処刑されました。",
                            self.players[target_index].name
                        ));

                        println!(
                            "AnonymousVoting processed successfully for target_id: {}",
                            target_id
                        );

                        // フェーズを更新
                        let from_phase = self.phase.clone();
                        self.phase = GamePhase::Result;
                        self.add_phase_change_message(from_phase, self.phase.clone());

                        // 4. 投票結果をクリア
                        self.vote_results.clear();

                        println!("Vote results cleared after processing.");
                    }
                    CircuitEncryptedInputIdentifier::WinningJudge(items) => {
                        // itemsを処理する
                    }
                    CircuitEncryptedInputIdentifier::RoleAssignment(items) => {
                        // itemsを処理する
                    }
                    CircuitEncryptedInputIdentifier::KeyPublicize(items) => {
                        // itemsを処理する
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

impl Serialize for CryptoParameters {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Dummy implementation
        unimplemented!("Serialization not implemented")
    }
}

impl<'de> Deserialize<'de> for CryptoParameters {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Dummy implementation
        unimplemented!("Deserialization not implemented")
    }
}
