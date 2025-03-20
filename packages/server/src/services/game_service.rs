use crate::{
    models::{
        game::{Game, GameAction, GamePhase, NightAction, NightActionRequest}, role::Role, room::RoomStatus
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
        match action {
            GameAction::StartGame => {
                game.phase = GamePhase::Night;
                Ok("ゲームを開始しました".to_string())
            }
            GameAction::EndGame => {
                game.phase = GamePhase::Finished;
                Ok("ゲームを終了しました".to_string())
            }
            GameAction::NextRole => {
                // 次のプレイヤーの役職を確認するためのアクション
                Ok("次のプレイヤーの役職を表示します".to_string())
            }
            GameAction::NextTurn => {
                // 次のターンに進むためのアクション
                force_next_phase(state.clone(), &room_id).await?;
                Ok("次のターンに進みました".to_string())
            }
        }
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
        // フェーズチェックを一時的に無効化（テストのため）
        // if game.phase != GamePhase::Voting {
        //     return Err("投票フェーズではありません".to_string());
        // }
        
        // プレイヤーIDを数値に変換
        let player_id = player_id.parse::<u32>()
            .map_err(|_| "無効なプレイヤーIDです".to_string())?;
            
        // プレイヤーが存在するか確認
        if !game.players.iter().any(|p| p.id == player_id) {
            return Err("プレイヤーが見つかりません".to_string());
        }
        
        // 投票を記録
        game.votes.insert(player_id, vote);
        Ok("投票を記録しました".to_string())
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

pub async fn process_night_action(
    state: AppState,
    room_id: &str,
    action_req: NightActionRequest,
) -> Result<String, String> {
    let mut games = state.games.lock().await;
    let game = games.get_mut(room_id).ok_or("Game not found")?;

    // フェーズチェック
    if game.phase != GamePhase::Night {
        return Err("夜のアクションは夜にのみ実行できます".to_string());
    }

    // プレイヤーIDを数値に変換
    let player_id = action_req.player_id.parse::<u32>()
        .map_err(|_| "無効なプレイヤーID形式です".to_string())?;
    
    // プレイヤーの存在確認と役職チェック
    let player = game.players.iter()
        .find(|p| p.id == player_id)
        .ok_or("プレイヤーが見つかりません")?;
    
    match (&player.role, &action_req.action) {
        (Role::Werewolf, NightAction::Attack { target_id }) => {
            // 襲撃対象のIDを数値に変換
            let target = target_id.parse::<u32>()
                .map_err(|_| "無効な対象プレイヤーID形式です".to_string())?;
            
            // 対象プレイヤーの存在確認
            if !game.players.iter().any(|p| p.id == target) {
                return Err("対象プレイヤーが見つかりません".to_string());
            }
            
            game.register_attack(&target.to_string())?;
            Ok("襲撃先を登録しました".to_string())
        }
        (Role::Seer, NightAction::Divine { target_id }) => {
            // 占い対象のIDを数値に変換
            let target = target_id.parse::<u32>()
                .map_err(|_| "無効な対象プレイヤーID形式です".to_string())?;
                
            // 対象プレイヤーの存在確認
            if !game.players.iter().any(|p| p.id == target) {
                return Err("対象プレイヤーが見つかりません".to_string());
            }

            let result = game.divine_player(&target.to_string())?;
            Ok(format!("プレイヤーの役職は {} です", result))
        }
        (Role::Guard, NightAction::Guard { target_id }) => {
            // 護衛対象のIDを数値に変換
            let target = target_id.parse::<u32>()
                .map_err(|_| "無効な対象プレイヤーID形式です".to_string())?;
                
            // 対象プレイヤーの存在確認
            if !game.players.iter().any(|p| p.id == target) {
                return Err("対象プレイヤーが見つかりません".to_string());
            }

            game.register_guard(&target.to_string())?;
            Ok("護衛先を登録しました".to_string())
        }
        _ => Err("このプレイヤーの役職ではこのアクションを実行できません".to_string()),
    }
}
