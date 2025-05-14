use axum::body::to_bytes;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use server::utils::test_setup::setup_test_env;
use server::{
    app::create_app,
    models::node::{NodeKey, RegisterKeyResponse},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_register_node_key() {
    setup_test_env();
    let app = create_app();

    let request_body = serde_json::json!({
        "node_id": 1,
        "public_key": "test-public-key"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/nodes/keys")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response: RegisterKeyResponse = serde_json::from_slice(&body).unwrap();
    assert!(response.success);
    assert_eq!(response.node_id, 1);
    assert_eq!(response.public_key, "test-public-key");
}

#[tokio::test]
async fn test_get_node_key() {
    setup_test_env();
    let app = create_app();

    // まず公開鍵を登録
    let register_body = serde_json::json!({
        "node_id": 1,
        "public_key": "test-public-key"
    });

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/nodes/keys")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&register_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap(); // 登録した公開鍵を取得
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/nodes/keys/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response: NodeKey = serde_json::from_slice(&body).unwrap();
    assert_eq!(response.node_id, 1);
    assert_eq!(response.public_key, "test-public-key");
}

#[tokio::test]
async fn test_invalid_node_id() {
    setup_test_env();
    let app = create_app();

    let request_body = serde_json::json!({
        "node_id": 4,  // 無効なノードID
        "public_key": "test-public-key"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/nodes/keys")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
