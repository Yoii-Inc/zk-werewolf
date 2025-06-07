use ark_bls12_377::Fr;
use ark_crypto_primitives::commitment::constraints::CommitmentGadget;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use ark_ff::UniformRand;
use ark_serialize::CanonicalDeserialize;
use ark_std::test_rng;
use mpc_algebra::CommitmentScheme;
use mpc_algebra::FromLocal;
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
async fn test_werewolf_vote_proof() {
    let rng = &mut test_rng();
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng).unwrap();

    let mpc_pedersen_param = <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(&pedersen_param);

    let player_randomness = (0..3).map(|_| Fr::rand(rng)).collect::<Vec<_>>();
    let player_commitment = player_randomness
        .iter()
        .map(|x| {
            <Fr as LocalOrMPC<Fr>>::PedersenComScheme::commit(
                &pedersen_param,
                &x.into_repr().to_bytes_le(),
                &<Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let mpc_player_commitment = player_commitment
        .iter()
        .map(|x| <MFr as LocalOrMPC<MFr>>::PedersenCommitment::from_public(*x))
        .collect::<Vec<_>>();

    let votes_data = vec![
        vec![
            MFr::from_public(Fr::from(0)),
            MFr::from_public(Fr::from(1)),
            MFr::from_public(Fr::from(0)),
        ],
        vec![
            MFr::from_public(Fr::from(0)),
            MFr::from_public(Fr::from(1)),
            MFr::from_public(Fr::from(0)),
        ],
        vec![
            MFr::from_public(Fr::from(0)),
            MFr::from_public(Fr::from(0)),
            MFr::from_public(Fr::from(1)),
        ],
    ];

    // 投票に関する入力データ
    let circuit = AnonymousVotingCircuit {
        is_target_id: votes_data, // 投票対象のID
        pedersen_param: mpc_pedersen_param,
        player_randomness: player_randomness
            .iter()
            .map(|x| MFr::from_public(*x))
            .collect::<Vec<_>>(),
        player_commitment: mpc_player_commitment,
    };

    // 証明のリクエスト
    let proof_id = request_proof_with_output(
        CircuitIdentifier::Built(BuiltinCircuit::AnonymousVoting(circuit)),
        ProofOutputType::Public,
    )
    .await;

    // 30秒待つ
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    assert!(
        proof_id.is_ok(),
        "証明のリクエストに失敗: {:?}",
        proof_id.err()
    );

    // 証明の状態を確認
    let (status, output) = check_proof_status(&proof_id.unwrap()).await.unwrap();
    assert!(status, "証明の生成に失敗");
    assert!(output.is_some(), "証明の出力が存在しません");

    let a = output.unwrap();
    assert!(a.value.is_some(), "証明の出力が存在しません");

    let expected_output = Fr::from(1);
    let actual_output: Fr = CanonicalDeserialize::deserialize(&mut &a.value.unwrap()[..]).unwrap();

    // expected player 1 is most voted.
    assert_eq!(actual_output, expected_output);
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
