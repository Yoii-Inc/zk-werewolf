use server::models::game::GamePhase;

use server::{
    models::{player::Player, room::Room},
    services::game_service,
    state::AppState,
    utils::test_setup::setup_test_env,
};

/// テスト用のルームにプレイヤーを追加（役職情報なし）
async fn setup_test_room_with_players(state: &AppState) -> String {
    let room_id = "test_room".to_string();

    // プレイヤーを4人作成（役職はサーバーでは管理しない）
    let players = vec![
        Player {
            id: "1".to_string(),
            name: "Player1".to_string(),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "2".to_string(),
            name: "Player2".to_string(),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "3".to_string(),
            name: "Player3".to_string(),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "4".to_string(),
            name: "Player4".to_string(),
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
async fn test_game_start() {
    println!("Testing game start");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    let start_result = game_service::start_game(state.clone(), &room_id).await;
    assert!(start_result.is_ok(), "ゲーム開始に失敗: {:?}", start_result);

    // ゲーム状態を確認
    let game_state = game_service::get_game_state(state.clone(), room_id.clone())
        .await
        .unwrap();

    // 夜フェーズで開始することを確認
    assert_eq!(game_state.phase, GamePhase::Night);

    // プレイヤー数が維持されていることを確認
    assert_eq!(game_state.players.len(), 4);

    // 暗号パラメータが初期化されていることを確認
    assert!(game_state.crypto_parameters.is_some());

    println!("Game started successfully in Night phase");
}

#[tokio::test]
async fn test_phase_transitions() {
    println!("Testing phase transitions");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    game_service::start_game(state.clone(), &room_id)
        .await
        .unwrap();

    // Night → DivinationProcessing
    let result = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(result.is_ok());
    assert_eq!(
        game_service::get_game_state(state.clone(), room_id.clone())
            .await
            .unwrap()
            .phase,
        GamePhase::DivinationProcessing
    );

    // DivinationProcessing → Discussion
    let result = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(result.is_ok());
    assert_eq!(
        game_service::get_game_state(state.clone(), room_id.clone())
            .await
            .unwrap()
            .phase,
        GamePhase::Discussion
    );

    // Discussion → Voting
    let result = game_service::advance_game_phase(state.clone(), &room_id).await;
    assert!(result.is_ok());
    assert_eq!(
        game_service::get_game_state(state.clone(), room_id.clone())
            .await
            .unwrap()
            .phase,
        GamePhase::Voting
    );

    println!("Phase transitions (Night → Voting) completed successfully");
}

// 投票システムのテストは削除（投票処理はMPC側で実装されているため）
// 投票結果の処理は外部のMPCノードから送信される計算結果によって決定される

// 勝利判定のテストは削除（勝利判定はMPC側で実装されている）
// 勝利判定結果は外部のMPCノードから送信される計算結果によって決定される

#[tokio::test]
async fn test_game_end() {
    println!("Testing game end");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    game_service::start_game(state.clone(), &room_id)
        .await
        .unwrap();

    // ゲーム終了
    let end_result = game_service::end_game(state.clone(), room_id.clone()).await;
    assert!(end_result.is_ok());

    // ゲームがFinishedフェーズになっていることを確認（削除されない）
    let game_state = game_service::get_game_state(state.clone(), room_id).await;
    assert!(game_state.is_ok());
    assert_eq!(game_state.unwrap().phase, GamePhase::Finished);

    println!("Game ended successfully");
}

#[tokio::test]
async fn test_initial_player_state() {
    println!("Testing initial player state");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    game_service::start_game(state.clone(), &room_id)
        .await
        .unwrap();

    // 初期状態で全員生存していることを確認
    {
        let games = state.games.lock().await;
        let game = games.get(&room_id).unwrap();

        // 全プレイヤーが生存していることを確認
        for player in &game.players {
            assert!(
                !player.is_dead,
                "Player {} should be alive initially",
                player.id
            );
        }

        // プレイヤー数が正しいことを確認
        assert_eq!(game.players.len(), 4);
    }

    println!("Initial player state is correct");
}

#[tokio::test]
async fn test_batch_request_initialization() {
    println!("Testing batch request initialization");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    game_service::start_game(state.clone(), &room_id)
        .await
        .unwrap();

    // バッチリクエストが初期化されていることを確認
    {
        let games = state.games.lock().await;
        let game = games.get(&room_id).unwrap();

        assert!(game.batch_request.requests.is_empty());
        assert_eq!(
            game.batch_request.status,
            server::models::game::BatchStatus::Collecting
        );
    }

    println!("Batch request initialized correctly");
}

#[tokio::test]
async fn test_crypto_parameters_initialization() {
    println!("Testing crypto parameters initialization");
    setup_test_env();
    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    game_service::start_game(state.clone(), &room_id)
        .await
        .unwrap();

    // 暗号パラメータが初期化されていることを確認
    {
        let games = state.games.lock().await;
        let game = games.get(&room_id).unwrap();

        assert!(game.crypto_parameters.is_some());

        let crypto_params = game.crypto_parameters.as_ref().unwrap();
        // プレイヤー数分のスロットが用意されていることを確認
        assert!(crypto_params.player_commitment.len() <= game.players.len());
    }

    println!("Crypto parameters initialized correctly");
}
