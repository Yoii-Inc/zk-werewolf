use std::time::Duration;

// TODO: ゲームルールの設定機能の実装
// 以下の機能を実装予定：
// - ゲーム開始時のプレイヤー数設定
// - 各役職の人数バランス設定
// - 各フェーズ（議論/投票）の制限時間設定
// - 夜アクションの制限設定（例：占い師は一晩に一人のみ占える）
// - 投票システムの設定（過半数必要、同数の場合はランダム等）

#[derive(Debug)]
#[allow(dead_code)]
pub struct Rule {
    pub max_players: usize,
    pub phase_duration: Duration,
    pub vote_threshold: usize,
}
