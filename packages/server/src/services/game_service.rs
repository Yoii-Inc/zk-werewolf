use crate::{
    models::{
        game::{Game, GamePhase, GameResult, NightAction, NightActionRequest},
        role::Role,
        room::RoomStatus,
    },
    state::AppState,
};

// ゲームのライフサイクル管理
pub async fn start_game(state: AppState, room_id: &str) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        let new_game = Game::new(room_id.to_string(), room.players.clone());
        state
            .games
            .lock()
            .await
            .insert(room_id.to_string(), new_game);
        room.status = RoomStatus::InProgress;

        // ゲーム開始後、最初のフェーズに進める
        advance_game_phase(state.clone(), room_id).await?;

        Ok("Game started successfully".to_string())
    } else {
        Err("Room not found".to_string())
    }
}

pub async fn end_game(state: AppState, room_id: String) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;
    let mut games = state.games.lock().await;
    if let (Some(room), Some(game)) = (rooms.get_mut(&room_id), games.get_mut(&room_id)) {
        room.status = RoomStatus::Closed;
        game.phase = GamePhase::Finished;
        Ok("Game ended successfully".to_string())
    } else {
        Err("Game not found".to_string())
    }
}

pub async fn get_game_state(state: AppState, room_id: String) -> Result<Game, String> {
    let games = state.games.lock().await;
    // if let Some(game) = games.get(&room_id) {
    //     Ok(game.to_string())
    // } else {
    //     Err("Game not found".to_string())
    // }
    games
        .get(&room_id)
        .cloned()
        .ok_or("Game not found".to_string())
}

// ゲームフェーズ管理
pub async fn advance_game_phase(state: AppState, room_id: &str) -> Result<String, String> {
    let mut games = state.games.lock().await;
    if let Some(game) = games.get_mut(room_id) {
        game.phase = match game.phase {
            GamePhase::Waiting => GamePhase::Night,
            GamePhase::Night => {
                game.resolve_night_actions();
                GamePhase::Discussion
            }
            GamePhase::Discussion => GamePhase::Voting,
            GamePhase::Voting => {
                game.resolve_voting();
                GamePhase::Result
            }
            GamePhase::Result => GamePhase::Night,
            GamePhase::Finished => return Err("ゲームは既に終了しています".to_string()),
        };
        Ok(format!("フェーズを更新しました: {:?}", game.phase))
    } else {
        Err("Game not found".to_string())
    }
}

// 夜のアクション処理
pub async fn process_night_action(
    state: AppState,
    room_id: &str,
    action_req: NightActionRequest,
) -> Result<String, String> {
    let mut games = state.games.lock().await;
    let game = games.get_mut(room_id).ok_or("Game not found")?;

    if game.phase != GamePhase::Night {
        return Err("夜のアクションは夜にのみ実行できます".to_string());
    }

    let player = game
        .players
        .iter()
        .find(|p| p.id == action_req.player_id)
        .ok_or("プレイヤーが見つかりません")?;

    match (
        player.role.as_ref().unwrap_or(&Role::Villager),
        &action_req.action,
    ) {
        (Role::Werewolf, NightAction::Attack { target_id }) => {
            game.register_attack(target_id)?;
            Ok("襲撃先を登録しました".to_string())
        }
        (Role::Seer, NightAction::Divine { target_id }) => {
            let result = game.divine_player(target_id)?;
            Ok(format!("プレイヤーの役職は {} です", result))
        }
        (Role::Guard, NightAction::Guard { target_id }) => {
            game.register_guard(target_id)?;
            Ok("護衛先を登録しました".to_string())
        }
        _ => Err("このプレイヤーの役職ではこのアクションを実行できません".to_string()),
    }
}

// 投票システム
pub async fn handle_vote(
    state: AppState,
    room_id: &str,
    voter_id: &str,
    target_id: &str,
) -> Result<String, String> {
    let mut games = state.games.lock().await;
    let game = games.get_mut(room_id).ok_or("Game not found")?;

    if game.phase != GamePhase::Voting {
        return Err("現在は投票フェーズではありません".to_string());
    }

    game.cast_vote(voter_id, target_id)?;
    Ok("投票を受け付けました".to_string())
}

// 勝利判定
pub async fn check_winner(state: AppState, room_id: &str) -> Result<GameResult, String> {
    let mut games = state.games.lock().await;
    let game = games.get_mut(room_id).ok_or("Game not found")?;

    // 生存者のみをカウント
    let living_players: Vec<_> = game.players.iter().filter(|p| !p.is_dead).collect();

    let alive_villagers = living_players
        .iter()
        .filter(|p| p.role.as_ref() != Some(&Role::Werewolf))
        .count();

    let alive_werewolves = living_players
        .iter()
        .filter(|p| p.role.as_ref() == Some(&Role::Werewolf))
        .count();

    let result = if alive_werewolves == 0 {
        GameResult::VillagerWin
    } else if alive_werewolves >= alive_villagers {
        GameResult::WerewolfWin
    } else {
        GameResult::InProgress
    };

    game.result = result.clone();

    if result != GameResult::InProgress {
        let mut rooms = state.rooms.lock().await;
        let room = rooms.get_mut(room_id).ok_or("Room not found")?;
        room.status = RoomStatus::Closed;
    }

    Ok(result)
}
