use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{player::Player, room::RoomStatus};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub room_id: String,
    pub name: String,
    pub players: Vec<Player>,
    pub max_players: usize,
    pub roles: Vec<String>,
    pub phase: GamePhase,
    pub result: GameResult,
    pub votes: HashMap<u32, bool>, // 追加: プレイヤーIDと投票内容のマップ
    pub night_actions: NightActions, // 追加: 夜のアクション記録
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NightActions {
    pub attacks: Vec<String>,  // 襲撃対象
    pub guards: Vec<String>,   // 護衛対象
    pub divinations: Vec<String>, // 占い対象
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Game {{ room_id: {}, name: {}, players: {:?}, max_players: {}, roles: {:?}, phase: {:?}, result: {:?} }}", 
            self.room_id, self.name, self.players, self.max_players, self.roles, self.phase, self.result)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GamePhase {
    Waiting,        // ゲーム開始前
    Night,          // 夜フェーズ
    Discussion,     // 議論フェーズ
    Voting,         // 投票フェーズ
    Result,         // 結果発表フェーズ
    Finished        // ゲーム終了
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameResult {
    InProgress,
    VillagerWin,    // 村人陣営勝利
    WerewolfWin,    // 人狼陣営勝利
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NightAction {
    Attack { target_id: String },    // 人狼の襲撃
    Divine { target_id: String },    // 占い師の占い
    Guard { target_id: String },     // 騎士の護衛
    // 必要に応じて追加の役職アクション
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NightActionRequest {
    pub player_id: String,
    pub action: NightAction,
}

impl Game {
    pub fn new(room_id: String, players: Vec<Player>) -> Self {
        Game {
            room_id,
            name: "".to_string(),
            players,
            max_players: 9,
            roles: vec![],
            phase: GamePhase::Waiting,
            result: GameResult::InProgress,
            votes: HashMap::new(),
            night_actions: NightActions::default(),
        }
    }

    pub fn register_attack(&mut self, target_id: &str) -> Result<(), String> {
        self.night_actions.attacks.push(target_id.to_string());
        Ok(())
    }

    pub fn divine_player(&self, target_id: &str) -> Result<String, String> {
        let target = self.players
            .iter()
            .find(|p| p.id.to_string() == target_id)
            .ok_or("対象プレイヤーが見つかりません")?;
        Ok(target.role.to_string())
    }

    pub fn register_guard(&mut self, target_id: &str) -> Result<(), String> {
        self.night_actions.guards.push(target_id.to_string());
        Ok(())
    }

    pub fn resolve_night_actions(&mut self) {
        use std::collections::HashSet;
        
        // 護衛成功判定
        let protected_players: HashSet<_> = self.night_actions.guards.iter().collect();
        
        // 襲撃処理（護衛されていない場合のみ）
        for target_id in &self.night_actions.attacks {
            if !protected_players.contains(target_id) {
                if let Some(player) = self.players
                    .iter_mut()
                    .find(|p| p.id.to_string() == *target_id) 
                {
                    player.is_dead = true;
                }
            }
        }

        // 夜アクションをリセット
        self.night_actions = NightActions::default();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    StartGame,
    EndGame,
    NextRole,
    NextTurn,
}
