use crate::{
    models::{
        game::{Game, GameAction, GamePhase},
        room::RoomStatus,
    },
    state::AppState,
};

pub async fn start_game(
    state: AppState,
    room_id: &str,
) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        let new_game = Game::new(room_id.to_string(), room.players.clone());
        state.games.lock().await.insert(room_id.to_string(), new_game);
        room.status = RoomStatus::InProgress;
        Ok("Game started successfully".to_string())
    } else {
        Err("Room not found".to_string())
    }
}

pub async fn get_game_state(
    state: AppState,
    room_id: String,
) -> Result<String, String> {
    let games = state.games.lock().await;
    if let Some(game) = games.get(&room_id) {
        Ok(game.to_string())
    } else {
        Err("Game not found".to_string())
    }
}

pub async fn post_game_action(
    state: AppState,
    room_id: String,
    action: GameAction,
    
) -> Result<String, String> {
    let mut games = state.games.lock().await;
    if let Some(game) = games.get_mut(&room_id) {
        // game.apply_action(action);
        todo!();
        Ok("Game action applied successfully".to_string())
    } else {
        Err("Game not found".to_string())
    }
}

pub async fn post_vote(
    state: AppState,
    room_id: String,
    player_id: String,
    vote: bool,
) -> Result<String, String> {
    let mut games = state.games.lock().await;
    if let Some(game) = games.get_mut(&room_id) {
        // game.vote(player_id, vote);
        Ok("Vote cast successfully".to_string())
    } else {
        Err("Game not found".to_string())
    }
}

// デバッグ用：次のフェーズに強制的に進める
pub async fn force_next_phase(state: AppState, room_id: &str) -> Result<String, String> {
    let mut games = state.games.lock().await;
    if let Some(game) = games.get_mut(room_id) {
        game.phase = match game.phase {
            GamePhase::Waiting => GamePhase::Night,
            GamePhase::Night => GamePhase::Discussion,
            GamePhase::Discussion => GamePhase::Voting,
            GamePhase::Voting => GamePhase::Result,
            GamePhase::Result => GamePhase::Night, // 結果フェーズから夜フェーズへ
            GamePhase::Finished => return Err("ゲームは既に終了しています".to_string()),
        };
        Ok(format!("フェーズを更新しました: {:?}", game.phase))
    } else {
        Err("Game not found".to_string())
    }
}

pub async fn end_game(
    state: AppState,
    room_id: String,
) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;
    let mut games = state.games.lock().await;
    if let (Some(room), Some(game)) = (rooms.get_mut(&room_id), games.get_mut(&room_id)) {
        room.status = RoomStatus::Closed;
        Ok("Game ended successfully".to_string())
    } else {
        Err("Game not found".to_string())
    }
}
