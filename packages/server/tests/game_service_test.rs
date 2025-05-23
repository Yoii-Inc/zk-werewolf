use server::models::game::GamePhase;

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

async fn setup_test_room_with_players(state: &AppState) -> String {
    let room_id = "test_room".to_string();
    // プレイヤーを4人作成（村人2人、占い師1人、人狼1人）
    let players = vec![
        Player {
            id: "1".to_string(),
            name: "Player1".to_string(),
            role: Some(Role::Villager),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "2".to_string(),
            name: "Player2".to_string(),
            role: Some(Role::Seer),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "3".to_string(),
            name: "Player3".to_string(),
            role: Some(Role::Werewolf),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "4".to_string(),
            name: "Player4".to_string(),
            role: Some(Role::Villager),
            is_dead: false,
            is_ready: false,
        },
    ];

    let mut room = Room::new(room_id.clone(), Some("Test Room".to_string()), Some(4));
    room.players = players;
    state.rooms.lock().await.insert(room_id.clone(), room);

    room_id
}

#[tokio::test]
async fn test_complete_game_flow() {
    println!("Starting game flow test");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    println!("Starting game in room: {}", room_id);
    let start_result = game_service::start_game(state.clone(), &room_id).await;
    assert!(start_result.is_ok(), "ゲーム開始に失敗: {:?}", start_result);

    println!("Game started successfully");

    // 夜フェーズであることを確認
    assert_eq!(
        game_service::get_game_state(state.clone(), room_id.clone())
            .await
            .unwrap()
            .phase,
        GamePhase::Night
    );

    // 占い師のプレイヤーのIDを取得
    let seer_id = state
        .games
        .lock()
        .await
        .get(&room_id)
        .unwrap()
        .players
        .iter()
        .find(|p| p.role == Some(Role::Seer))
        .unwrap()
        .id
        .clone();
    println!("Seer ID: {}", seer_id);

    // 人狼のプレイヤーのIDを取得
    let werewolf_id = state
        .games
        .lock()
        .await
        .get(&room_id)
        .unwrap()
        .players
        .iter()
        .find(|p| p.role == Some(Role::Werewolf))
        .unwrap()
        .id
        .clone();

    println!(
        "players: {:?}",
        state.games.lock().await.get(&room_id).unwrap().players
    );

    // 占い師（Player2）がPlayer3（人狼）を占う
    let night_action = NightActionRequest {
        player_id: seer_id.to_string(),
        action: NightAction::Divine {
            target_id: werewolf_id.to_string(),
        },
    };
    let divine_result =
        game_service::process_night_action(state.clone(), &room_id, night_action).await;
    assert!(
        divine_result.is_ok(),
        "占い結果の処理に失敗: {:?}",
        divine_result
    );
    let result_text = divine_result.unwrap();
    assert!(
        result_text.contains("人狼"),
        "占い結果が正しくありません: {}",
        result_text
    );

    // 議論フェーズへ
    let to_discussion = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_discussion.is_ok());
    assert_eq!(
        game_service::get_game_state(state.clone(), room_id.clone())
            .await
            .unwrap()
            .phase,
        GamePhase::Discussion
    );

    // 投票フェーズへ
    let to_voting = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(to_voting.is_ok());
    assert_eq!(
        game_service::get_game_state(state.clone(), room_id.clone())
            .await
            .unwrap()
            .phase,
        GamePhase::Voting
    );

    // 全員がPlayer4に投票
    for voter_id in 1..=4 {
        let vote_result =
            game_service::handle_vote(state.clone(), &room_id, &voter_id.to_string(), "4").await;
        vote_result.expect("投票処理に失敗");
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
