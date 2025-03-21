use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::json;
use server::app;
use tower::ServiceExt;

use server::utils::test_setup::setup_test_env;

#[tokio::test]
async fn test_create_room() {
    setup_test_env();
    let app = app::create_app();

    let request = Request::builder()
        .method("POST")
        .uri("/api/room/create")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Room created with ID:"));
}

#[tokio::test]
async fn test_join_room() {
    setup_test_env();
    let app = app::create_app();

    // まずルームを作成
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/room/create")
        .body(Body::empty())
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let room_id = body_str
        .replace("\"Room created with ID: ", "")
        .replace("\"", "");

    // ルーム参加のリクエストを送信
    let join_request = Request::builder()
        .method("POST")
        .uri(format!("/api/room/{}/join/1", room_id))
        .body(Body::empty())
        .unwrap();

    let join_response = app.oneshot(join_request).await.unwrap();

    assert_eq!(join_response.status(), StatusCode::OK);
    let join_body = to_bytes(join_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let join_body_str = String::from_utf8(join_body.to_vec()).unwrap();
    assert_eq!(join_body_str, "\"Successfully joined room\"");
}

#[tokio::test]
async fn test_voting_system() {
    setup_test_env();
    let app = app::create_app();

    // ルーム作成
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/room/create")
        .body(Body::empty())
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    let body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let room_id = body_str
        .replace("\"Room created with ID: ", "")
        .replace("\"", "");

    // プレイヤー1と2を参加させる
    for player_id in 1..=2 {
        let join_request = Request::builder()
            .method("POST")
            .uri(format!("/api/room/{}/join/{}", room_id, player_id))
            .body(Body::empty())
            .unwrap();
        let join_response = app.clone().oneshot(join_request).await.unwrap();
        assert_eq!(join_response.status(), StatusCode::OK);
    }

    // ゲームを開始
    let start_game_request = Request::builder()
        .method("POST")
        .uri(format!("/api/game/{}/start", room_id))
        .body(Body::empty())
        .unwrap();
    let start_game_response = app.clone().oneshot(start_game_request).await.unwrap();
    assert_eq!(start_game_response.status(), StatusCode::OK);

    // 各フェーズでの状態を確認
    let phases = ["Night", "Discussion", "Voting"];
    for phase_name in phases.iter() {
        // 現在のフェーズを確認
        let state_request = Request::builder()
            .method("GET")
            .uri(format!("/api/game/{}/state", room_id))
            .body(Body::empty())
            .unwrap();
        let state_response = app.clone().oneshot(state_request).await.unwrap();
        assert_eq!(state_response.status(), StatusCode::OK);

        // 次のフェーズに進める
        let next_phase_request = Request::builder()
            .method("POST")
            .uri(format!("/api/game/{}/phase/next", room_id))
            .body(Body::empty())
            .unwrap();
        let phase_response = app.clone().oneshot(next_phase_request).await.unwrap();
        assert_eq!(
            phase_response.status(),
            StatusCode::OK,
            "フェーズ '{}' への移行に失敗",
            phase_name
        );
    }

    // プレイヤー1がプレイヤー2に投票
    let vote_action = json!({
        "voter_id": "1",
        "target_id": "2"
    });

    let cast_vote_request = Request::builder()
        .method("POST")
        .uri(format!("/api/game/{}/actions/vote", room_id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&vote_action).unwrap()))
        .unwrap();
    let cast_vote_response = app.clone().oneshot(cast_vote_request).await.unwrap();
    let vote_status = cast_vote_response.status();
    let vote_body = to_bytes(cast_vote_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vote_body_str = String::from_utf8(vote_body.to_vec()).unwrap();
    assert_eq!(vote_status, StatusCode::OK, "投票に失敗: {}", vote_body_str);

    // 結果フェーズに進める
    let next_phase_request = Request::builder()
        .method("POST")
        .uri(format!("/api/game/{}/phase/next", room_id))
        .body(Body::empty())
        .unwrap();
    let next_phase_response = app.oneshot(next_phase_request).await.unwrap();
    assert_eq!(next_phase_response.status(), StatusCode::OK);

    let next_phase_body = to_bytes(next_phase_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let next_phase_body_str = String::from_utf8(next_phase_body.to_vec()).unwrap();
    assert!(
        next_phase_body_str.contains("フェーズを更新しました"),
        "フェーズの更新に失敗: {}",
        next_phase_body_str
    );
}
