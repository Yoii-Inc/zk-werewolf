use super::player::Player;
use ark_bls12_377::Fr;
use ark_crypto_primitives::{encryption::AsymmetricEncryptionScheme, CommitmentScheme};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

#[derive(Serialize, Deserialize, Debug, Clone)]
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
