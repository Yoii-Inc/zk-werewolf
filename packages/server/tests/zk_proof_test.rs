use ark_bls12_377::Fr;
use mpc_algebra::Reveal;
use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use server::{
    models::player,
    services::zk_proof::{check_proof_status, request_proof_with_output},
};
use zk_mpc::{
    circuits::{circuit::MySimpleCircuit, AnonymousVotingCircuit, LocalOrMPC},
    marlin::MFr,
};
use zk_mpc_node::{BuiltinCircuit, CircuitIdentifier, ProofOutputType};

// TODO: fix this test
#[tokio::test]
async fn test_request_proof() {
    let fa = MFr::from_public(Fr::from(2));
    let fb = MFr::from_public(Fr::from(3));

    let circuit = MySimpleCircuit {
        a: Some(fa),
        b: Some(fb),
    };

    let result = request_proof_with_output(
        zk_mpc_node::CircuitIdentifier::Built(zk_mpc_node::BuiltinCircuit::MySimple(circuit)),
        zk_mpc_node::ProofOutputType::Public,
    )
    .await;
    assert!(
        result.is_ok(),
        "Failed to request proof: {:?}",
        result.err()
    );

    let proof_id = result.unwrap();

    let status = check_proof_status(&proof_id).await;
    assert!(
        status.is_ok(),
        "Failed to check proof status: {:?}",
        status.err()
    );
}

#[tokio::test]
#[ignore]
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
    assert!(result.unwrap().0);
}

#[tokio::test]
#[ignore]
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
    assert!(!result.unwrap().0);
}

#[tokio::test]
#[ignore]
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
    assert!(
        result.is_ok(),
        "Failed to check proof status: {:?}",
        result.err()
    );
    assert!(!result.unwrap().0);
}
