use std::env;

#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub enabled: bool,
    pub verbose_logging: bool,
    // プレイヤーの役職を表示するかどうか
    pub show_player_roles: bool,
    // 自動でフェーズを進めるかどうか
    pub auto_advance_phases: bool,
    pub phase_duration_seconds: u64,
    // proofを作るかどうか
    pub create_proof: bool,
    // proofをzk-mpcノードに委任するかどうか
    pub delegate_proof: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        let debug_mode = cfg!(debug_assertions) || env::var("DEBUG_MODE").is_ok();

        Self {
            enabled: debug_mode,
            verbose_logging: debug_mode,
            show_player_roles: debug_mode,
            auto_advance_phases: false,
            phase_duration_seconds: 30,
            create_proof: false,
            delegate_proof: false,
        }
    }
}

impl DebugConfig {
    pub fn from_env() -> Self {
        let enabled = env::var("DEBUG_ENABLED")
            .map(|v| v == "true")
            .unwrap_or_else(|_| cfg!(debug_assertions));
        let verbose_logging = env::var("DEBUG_VERBOSE_LOGGING")
            .map(|v| v == "true")
            .unwrap_or(enabled);
        let show_player_roles = env::var("DEBUG_SHOW_PLAYER_ROLES")
            .map(|v| v == "true")
            .unwrap_or(enabled);
        let auto_advance_phases = env::var("DEBUG_AUTO_ADVANCE_PHASES")
            .map(|v| v == "true")
            .unwrap_or(false);
        let phase_duration_seconds = env::var("DEBUG_PHASE_DURATION_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);

        let create_proof = env::var("DEBUG_CREATE_PROOF")
            .map(|v| v == "true")
            .unwrap_or(false);
        let delegate_proof = env::var("DEBUG_DELEGATE_PROOF")
            .map(|v| v == "true")
            .unwrap_or(false);

        Self {
            enabled,
            verbose_logging,
            show_player_roles,
            auto_advance_phases,
            phase_duration_seconds,
            create_proof,
            delegate_proof,
        }
    }
}
