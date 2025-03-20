use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use server::app;
use tower::ServiceExt;

#[tokio::test]
async fn test_create_room() {
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

    // 投票開始
    let start_vote_request = Request::builder()
        .method("POST")
        .uri(format!("/api/room/{}/vote/start", room_id))
        .body(Body::empty())
        .unwrap();
    let start_vote_response = app.clone().oneshot(start_vote_request).await.unwrap();
    assert_eq!(start_vote_response.status(), StatusCode::OK);

    // プレイヤー1がプレイヤー2に投票
    let cast_vote_request = Request::builder()
        .method("POST")
        .uri(format!("/api/room/{}/vote/cast/1/2", room_id))
        .body(Body::empty())
        .unwrap();
    let cast_vote_response = app.clone().oneshot(cast_vote_request).await.unwrap();
    assert_eq!(cast_vote_response.status(), StatusCode::OK);

    // 投票終了
    let end_vote_request = Request::builder()
        .method("POST")
        .uri(format!("/api/room/{}/vote/end", room_id))
        .body(Body::empty())
        .unwrap();
    let end_vote_response = app.oneshot(end_vote_request).await.unwrap();
    assert_eq!(end_vote_response.status(), StatusCode::OK);

    let end_vote_body = to_bytes(end_vote_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let end_vote_body_str = String::from_utf8(end_vote_body.to_vec()).unwrap();
    assert!(end_vote_body_str.contains("Player 2が最多得票"));
}
