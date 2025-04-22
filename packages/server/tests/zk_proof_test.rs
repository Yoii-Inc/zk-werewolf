use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use server::services::zk_proof::{check_proof_status, request_proof};

// TODO: fix this test
#[tokio::test]
async fn test_request_proof() {
    // モックサーバーのセットアップ
    let mock_server = MockServer::start().await;

    // 証明リクエストのモック
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "proof_id": "test-proof-123"
        })))
        .mount(&mock_server)
        .await;

    // テストケースの実行
    let circuit_inputs = json!({
        "voter_id": "1",
        "target_id": "2",
        "is_voter_alive": true,
        "is_target_alive": true,
        "is_voting_phase": true
    });

    let result = request_proof(
        zk_mpc_node::CircuitIdentifier::Built(zk_mpc_node::CircuitType::AnonymousVotingCircuit),
        circuit_inputs,
    )
    .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test-proof-123");
}

#[tokio::test]
async fn test_check_proof_status_completed() {
    let mock_server = MockServer::start().await;

    // 完了状態の証明ステータスのモック
    Mock::given(method("GET"))
        .and(path("/proof/test-proof-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "state": "completed"
        })))
        .mount(&mock_server)
        .await;

    let result = check_proof_status("test-proof-123").await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_check_proof_status_failed() {
    let mock_server = MockServer::start().await;

    // 失敗状態の証明ステータスのモック
    Mock::given(method("GET"))
        .and(path("/proof/test-proof-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "state": "failed"
        })))
        .mount(&mock_server)
        .await;

    let result = check_proof_status("test-proof-123").await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_check_proof_status_invalid_response() {
    let mock_server = MockServer::start().await;

    // 無効なレスポンスのモック
    Mock::given(method("GET"))
        .and(path("/proof/test-proof-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "state": "invalid_state"
        })))
        .mount(&mock_server)
        .await;

    let result = check_proof_status("test-proof-123").await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
