use tokio;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use tokio::sync::broadcast;

use server::{
    state::AppState,
    models::{
        game::{Game, GameAction, GamePhase, NightAction, NightActionRequest},
        player::Player,
        room::{Room, RoomStatus},
        role::Role,
    },
    services::game_service,
};

fn setup_test_state() -> AppState {
    let rooms = Arc::new(Mutex::new(HashMap::new()));
    let games = Arc::new(Mutex::new(HashMap::new()));
    let (tx, _) = broadcast::channel(100);
    AppState { 
        rooms,
        games,
        channel: tx,
    }
}

async fn setup_test_room(state: &AppState) -> String {
    let room_id = "test_room".to_string();
    let mut room = Room::new(
        room_id.clone(),
        Some("Test Room".to_string()),
        Some(5)
    );
    
    // テスト用のプレイヤーを追加
    room.players.push(Player {
        id: 1,
        name: "Player1".to_string(),
        role: Role::Werewolf,
        is_dead: false,
    });
    room.players.push(Player {
        id: 2,
        name: "Player2".to_string(),
        role: Role::Villager,
        is_dead: false,
    });
    room.players.push(Player {
        id: 3,
        name: "Player3".to_string(),
        role: Role::Seer,
        is_dead: false,
    });

    state.rooms.lock().await.insert(room_id.clone(), room);
    room_id
}

#[tokio::test]
async fn test_game_lifecycle() {
    let state = setup_test_state();
    let room_id = setup_test_room(&state).await;

    // ゲーム開始のテスト
    let start_result = game_service::start_game(state.clone(), &room_id).await;
    assert!(start_result.is_ok());
    
    // ゲーム状態の取得テスト
    let state_result = game_service::get_game_state(state.clone(), room_id.clone()).await;
    assert!(state_result.is_ok());

    // フェーズ遷移のテスト（夜フェーズへ）
    let next_phase_result = game_service::force_next_phase(state.clone(), &room_id).await;
    assert!(next_phase_result.is_ok());
    
    // 夜のアクションのテスト（人狼の襲撃）
    let night_action = NightActionRequest {
        player_id: "1".to_string(), // 人狼のプレイヤー
        action: NightAction::Attack { target_id: "2".to_string() }, // 村人を襲撃
    };
    let action_result = game_service::process_night_action(
        state.clone(),
        &room_id,
        night_action,
    ).await;
    assert!(action_result.is_ok(), "夜のアクションが失敗: {:?}", action_result);

    // 投票フェーズへの遷移
    let next_phase_result = game_service::force_next_phase(state.clone(), &room_id).await;
    assert!(next_phase_result.is_ok());

    // 投票のテスト
    let vote_result = game_service::post_vote(
        state.clone(),
        room_id.clone(),
        "1".to_string(),
        true,
    ).await;
    assert!(vote_result.is_ok());

    // ゲーム終了のテスト
    let end_result = game_service::end_game(state.clone(), room_id.clone()).await;
    assert!(end_result.is_ok());

    // ゲーム終了後の状態確認
    let rooms = state.rooms.lock().await;
    assert_eq!(rooms.get(&room_id).unwrap().status, RoomStatus::Closed);
}

#[tokio::test]
async fn test_error_cases() {
    let state = setup_test_state();
    let invalid_room_id = "nonexistent_room".to_string();

    // 存在しないルームでのゲーム開始
    let start_result = game_service::start_game(state.clone(), &invalid_room_id).await;
    assert!(start_result.is_err());

    // 存在しないゲームの状態取得
    let state_result = game_service::get_game_state(state.clone(), invalid_room_id.clone()).await;
    assert!(state_result.is_err());

    // 存在しないゲームでの投票
    let vote_result = game_service::post_vote(
        state.clone(),
        invalid_room_id.clone(),
        "1".to_string(),
        true,
    ).await;
    assert!(vote_result.is_err());

    // 存在しないゲームの終了
    let end_result = game_service::end_game(state.clone(), invalid_room_id.clone()).await;
    assert!(end_result.is_err());
}