use server::services::user_service;
use std::collections::HashMap;
use std::sync::Arc;
use tokio;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

use server::{
    models::{
        game::{GameResult, NightAction, NightActionRequest},
        player::Player,
        role::Role,
        room::Room,
    },
    services::game_service,
    state::AppState,
    utils::test_setup::setup_test_env,
};

fn setup_test_state() -> AppState {
    let rooms = Arc::new(Mutex::new(HashMap::new()));
    let games = Arc::new(Mutex::new(HashMap::new()));
    let (tx, _) = broadcast::channel(100);
    let user_service = user_service::UserService::new();
    AppState {
        rooms,
        games,
        channel: tx,
        user_service,
    }
}

async fn setup_test_room_with_players(state: &AppState) -> String {
    let room_id = "test_room".to_string();
    let mut players = vec![];

    // プレイヤーを4人作成（村人2人、占い師1人、人狼1人）
    players.push(Player {
        id: "1".to_string(),
        name: "Player1".to_string(),
        role: Some(Role::Villager),
        is_dead: false,
    });
    players.push(Player {
        id: "2".to_string(),
        name: "Player2".to_string(),
        role: Some(Role::Seer),
        is_dead: false,
    });
    players.push(Player {
        id: "3".to_string(),
        name: "Player3".to_string(),
        role: Some(Role::Werewolf),
        is_dead: false,
    });
    players.push(Player {
        id: "4".to_string(),
        name: "Player4".to_string(),
        role: Some(Role::Villager),
        is_dead: false,
    });

    let mut room = Room::new(room_id.clone(), Some("Test Room".to_string()), Some(4));
    room.players = players;
    state.rooms.lock().await.insert(room_id.clone(), room);

    room_id
}

#[tokio::test]
async fn test_complete_game_flow() {
    setup_test_env();
    let state = setup_test_state();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    let start_result = game_service::start_game(state.clone(), &room_id).await;
    assert!(start_result.is_ok(), "ゲーム開始に失敗: {:?}", start_result);

    // 夜フェーズに移行
    let phase_result = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(phase_result.is_ok());

    // 占い師（Player2）がPlayer3（人狼）を占う
    let night_action = NightActionRequest {
        player_id: "2".to_string(),
        action: NightAction::Divine {
            target_id: "3".to_string(),
        },
    };
    let divine_result =
        game_service::process_night_action(state.clone(), &room_id, night_action).await;
    assert!(divine_result.is_ok());
    let result_text = divine_result.unwrap();
    assert!(
        result_text.contains("人狼"),
        "占い結果が正しくありません: {}",
        result_text
    );

    // 議論フェーズへ
    let to_discussion = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_discussion.is_ok());

    // 投票フェーズへ
    let to_voting = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_voting.is_ok());

    // 全員がPlayer4に投票
    for voter_id in 1..=4 {
        let vote_result =
            game_service::handle_vote(state.clone(), &room_id, &voter_id.to_string(), "4").await;
        assert!(vote_result.is_ok());
    }

    // 結果フェーズへ（Player4が処刑される）
    let to_result = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_result.is_ok());

    // 勝利判定（まだゲームは続く）
    let winner_check1 = game_service::check_winner(state.clone(), &room_id).await;
    assert!(winner_check1.is_ok());
    assert!(matches!(winner_check1.unwrap(), GameResult::InProgress));

    // 夜フェーズへ
    let to_night = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_night.is_ok());

    // 占い師がPlayer1を占う
    let night_action2 = NightActionRequest {
        player_id: "2".to_string(),
        action: NightAction::Divine {
            target_id: "1".to_string(),
        },
    };
    let divine_result2 =
        game_service::process_night_action(state.clone(), &room_id, night_action2).await;
    assert!(divine_result2.is_ok());
    let result_text2 = divine_result2.unwrap();
    assert!(
        result_text2.contains("村人"),
        "占い結果が正しくありません: {}",
        result_text2
    );

    // 人狼がPlayer1を襲撃
    let night_action3 = NightActionRequest {
        player_id: "3".to_string(),
        action: NightAction::Attack {
            target_id: "1".to_string(),
        },
    };
    let attack_result =
        game_service::process_night_action(state.clone(), &room_id, night_action3).await;
    assert!(attack_result.is_ok());

    // 議論フェーズへ（Player1が死亡している）
    let to_discussion2 = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_discussion2.is_ok());

    // 最終的な勝利判定（人狼の勝利になるはず）
    let winner_check2 = game_service::check_winner(state.clone(), &room_id).await;
    assert!(winner_check2.is_ok());
    assert!(matches!(winner_check2.unwrap(), GameResult::WerewolfWin));

    // ゲーム終了
    let end_result = game_service::end_game(state.clone(), room_id).await;
    assert!(end_result.is_ok());
}
