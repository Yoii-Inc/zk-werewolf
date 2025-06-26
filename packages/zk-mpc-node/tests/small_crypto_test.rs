// Test five types of circuits based on the following:
// Setting: Start 3 nodes.
// 1. Prepare circuit inputs for each user
// 2. Split and encrypt inputs using mpc-algebra-wasm
// 3. Generate proofs using encrypted data
// 4. Verify proofs

// use mpc_algebra_wasm::*;

use ark_bls12_377::Fr;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use ark_ff::UniformRand;
use ark_std::test_rng;
use ark_std::PubUniformRand;
use base64::decode;
use crypto_box::PublicKey;
use crypto_box::SecretKey;
use mpc_algebra::CommitmentScheme;
use mpc_algebra::FromLocal;
use mpc_algebra::Reveal;
use mpc_algebra_wasm::AnonymousVotingCircuit;
use mpc_circuits::CircuitFactory;
use mpc_circuits::{BuiltinCircuit, CircuitIdentifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serial_test::serial;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;
use zk_mpc::circuits::circuit::MySimpleCircuit;
use zk_mpc::circuits::LocalOrMPC;
use zk_mpc::marlin::MFr;
use zk_mpc_node::KeyFile;
use zk_mpc_node::NodeKeys;
use zk_mpc_node::ProofStatus;
use zk_mpc_node::{ProofOutput, ProofOutputType, ProofRequest};

use mpc_algebra_wasm::*;

// use aes_gcm::{
//     aead::{Aead, OsRng},
//     Aes256Gcm, KeyInit,
// };
// use mpc_algebra_wasm::{
//     circuits::{Circuit, SimpleCircuit},
//     encryption::{encrypt_input, split_secret},
//     proof::{generate_proof, verify_proof},
// };

const USER_NUM: usize = 3;
const NODE_NUM: usize = 3;

fn get_node_keys() -> (Vec<NodeKey>, Vec<String>) {
    // data/node_keys_0.json, data/node_keys_1.json, data/node_keys_2.jsonから読み込む
    let mut keys = Vec::new();
    let mut secret_keys = Vec::new();
    for i in 0..NODE_NUM {
        // ファイルから読み込む
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let file_path = format!("{}/node_keys_{}.json", data_dir, i);
        let file = std::fs::File::open(file_path).unwrap();
        let reader = std::io::BufReader::new(file);
        let key_file: NodeKeys = serde_json::from_reader(reader).unwrap();

        // let bytes = decode(&key_file.public_key).unwrap();
        // let read_keys = PublicKey::from_slice(&bytes).unwrap();
        keys.push(NodeKey {
            // node_id: format!("node{}", i + 1),
            node_id: format!("{}", i),
            public_key: key_file.public_key,
        });
        secret_keys.push(key_file.secret_key);
    }

    // println!("Node keys: {:?}", keys);
    (keys, secret_keys)

    // Base64デコードしてPublicKeyに変換
    // let bytes = decode(&key_file.public_key).unwrap();
    // let read_keys = PublicKey::from_slice(&bytes).unwrap();
}

fn setup_voting() -> (CircuitEncryptedInputIdentifier, Vec<(SecretKey, PublicKey)>) {
    let scheme = SecretSharingScheme {
        total_shares: NODE_NUM,
        modulus: 97,
    };
    let rng = &mut test_rng();
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng).unwrap();

    let mut private_inputs = Vec::new();
    let mut user_keys = Vec::new();

    for i in 0..USER_NUM {
        let secret_key = SecretKey::generate(rng);
        let public_key = secret_key.public_key();
        user_keys.push((secret_key, public_key));

        let private_input: AnonymousVotingPrivateInput = AnonymousVotingPrivateInput {
            id: i,
            is_target_id: vec![Fr::from(0), Fr::from(1), Fr::from(0)],
            player_randomness: Fr::pub_rand(rng),
        };

        private_inputs.push(private_input);
    }

    let public_input = AnonymousVotingPublicInput {
        pedersen_param: pedersen_param.clone(),
        player_commitment: vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); USER_NUM],
    };

    // let encrypted_inputs = private_inputs
    //     .iter()
    //     .map(|private_input| {
    //         CircuitEncryptedInputIdentifier::AnonymousVoting(
    //             AnonymousVotingEncryption::create_encrypted_shares(&AnonymousVotingInput {
    //                 private_input: private_input.clone(),
    //                 public_input: public_input.clone(),
    //                 node_keys: get_node_keys(),
    //                 scheme: scheme.clone(),
    //             })
    //             .unwrap(),
    //         )
    //     })
    //     .collect::<Vec<_>>();

    let encrypted_inputs = CircuitEncryptedInputIdentifier::AnonymousVoting(
        private_inputs
            .iter()
            .map(|private_input| {
                AnonymousVotingEncryption::create_encrypted_shares(&AnonymousVotingInput {
                    private_input: private_input.clone(),
                    public_input: public_input.clone(),
                    node_keys: get_node_keys().0,
                    scheme: scheme.clone(),
                })
                .unwrap()
            })
            .collect::<Vec<_>>(),
    );

    for i in 0..NODE_NUM {
        let secret_key = get_node_keys().1[i].clone();
        let secret_key = SecretKey::from_slice(&decode(secret_key).unwrap()).unwrap();

        let file_public_key =
            PublicKey::from_slice(&decode(get_node_keys().0[i].public_key.clone()).unwrap())
                .unwrap();

        assert!(
            secret_key.public_key() == file_public_key,
            "Public key mismatch for node {}",
            i
        );
    }

    assert_eq!(get_node_keys().0.len(), NODE_NUM);

    let test_input = AnonymousVotingInput {
        private_input: private_inputs[0].clone(),
        public_input: public_input.clone(),
        node_keys: get_node_keys().0,
        scheme: scheme.clone(),
    };

    let test_encrypted =
        AnonymousVotingEncryption::create_encrypted_shares(&AnonymousVotingInput {
            private_input: private_inputs[0].clone(),
            public_input: public_input.clone(),
            node_keys: get_node_keys().0,
            scheme: scheme.clone(),
        })
        .unwrap();

    let test_decrypted_0 =
        AnonymousVotingEncryption::decrypt(&test_encrypted.shares[0], &get_node_keys().1[0])
            .unwrap();

    let test_decrypted_1 =
        AnonymousVotingEncryption::decrypt(&test_encrypted.shares[1], &get_node_keys().1[1])
            .unwrap();

    let test_decrypted_2 =
        AnonymousVotingEncryption::decrypt(&test_encrypted.shares[2], &get_node_keys().1[2])
            .unwrap();

    assert_eq!(
        (test_decrypted_0.player_randomness
            + test_decrypted_1.player_randomness
            + test_decrypted_2.player_randomness),
        test_input.private_input.player_randomness
    );

    assert_eq!(
        (test_decrypted_0.is_target_id[0]
            + test_decrypted_1.is_target_id[0]
            + test_decrypted_2.is_target_id[0]),
        test_input.private_input.is_target_id[0]
    );

    (encrypted_inputs, user_keys)

    // let mpc_pedersen_param = <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(&pedersen_param);

    // let player_randomness = (0..3).map(|_| Fr::rand(rng)).collect::<Vec<_>>();
    // // let player_commitment = load_random_commitment()?;
    // let player_commitment = player_randomness
    //     .iter()
    //     .map(|x| {
    //         <Fr as LocalOrMPC<Fr>>::PedersenComScheme::commit(
    //             &pedersen_param,
    //             &x.into_repr().to_bytes_le(),
    //             &<Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
    //         )
    //         .unwrap()
    //     })
    //     .collect::<Vec<_>>();

    // let mpc_player_commitment = player_commitment
    //     .iter()
    //     .map(|x| <MFr as LocalOrMPC<MFr>>::PedersenCommitment::from_public(*x))
    //     .collect::<Vec<_>>();

    // let votes_data = vec![
    //     vec![
    //         MFr::from_public(Fr::from(0)),
    //         MFr::from_public(Fr::from(1)),
    //         MFr::from_public(Fr::from(0)),
    //     ],
    //     vec![
    //         MFr::from_public(Fr::from(0)),
    //         MFr::from_public(Fr::from(1)),
    //         MFr::from_public(Fr::from(0)),
    //     ],
    //     vec![
    //         MFr::from_public(Fr::from(0)),
    //         MFr::from_public(Fr::from(0)),
    //         MFr::from_public(Fr::from(1)),
    //     ],
    // ];

    // // 投票に関する入力データ
    // let circuit = AnonymousVotingCircuit {
    //     is_target_id: votes_data, // 投票対象のID
    //     // is_most_voted_id: MFr::from_public(Fr::from(1)), // 最多得票者のID
    //     pedersen_param: mpc_pedersen_param, // テスト用にNoneを設定
    //     player_randomness: player_randomness
    //         .iter()
    //         .map(|x| MFr::from_public(*x))
    //         .collect::<Vec<_>>(),
    //     player_commitment: mpc_player_commitment,
    // };
}

// 回路の証明のテストを書く。リクエスト及び他のノードは起動しない(small_test)

#[tokio::test]
#[serial]
async fn test_mpc_node_proof_generation_voting() -> Result<(), Box<dyn std::error::Error>> {
    // let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();

    let (circuit_encrypted_input, _user_keys) = setup_voting();

    // // シリアライズ
    // let serialized =
    //     serde_json::to_vec(&circuit).map_err(|e| format!("シリアライズに失敗: {}", e))?;
    // println!(
    //     "シリアライズされたデータのサイズ: {} bytes",
    //     serialized.len()
    // );

    // // デシリアライズして検証
    // let deserialized: AnonymousVotingCircuit<MFr> =
    //     serde_json::from_slice(&serialized).map_err(|e| format!("デシリアライズに失敗: {}", e))?;

    // // シリアライズ前後でデータが一致することを確認
    // assert_eq!(
    //     circuit.is_target_id, deserialized.is_target_id,
    //     "is_target_idのシリアライズ/デシリアライズ結果が一致しません"
    // );

    let test_req = ProofRequest {
        proof_id: "test_proof_id".to_string(),
        circuit_type: circuit_encrypted_input.clone(),
        output_type: ProofOutputType::Public,
    };

    let hoge = serde_json::to_string(&test_req).unwrap();

    let fuga = serde_json::from_str::<ProofRequest>(&hoge).unwrap();

    assert_eq!(test_req.proof_id, fuga.proof_id);
    // assert_eq!(test_req.circuit_type, fuga.circuit_type);
    // assert_eq!(test_req.output_type, fuga.output_type);

    tokio::time::sleep(Duration::from_secs(10)).await;

    // Send requests to the three ports
    let requests = [9000, 9001, 9002].map(|port| {
        let client = client.clone();
        let circuit = circuit_encrypted_input.clone();
        async move {
            let response = client
                .post(format!("http://localhost:{}", port))
                .json(&ProofRequest {
                    proof_id: "test_proof_id".to_string(),
                    circuit_type: circuit.clone(),
                    output_type: ProofOutputType::Public,
                })
                .send()
                .await?;

            let response_body: serde_json::Value = response.json().await?;
            println!("Response from port {}: {:?}", port, response_body);

            Ok::<_, Box<dyn std::error::Error>>((port, response_body))
        }
    });

    for request in requests {
        let response = request.await?;
        println!("Response from port {}: {:?}", response.0, response.1);
    }

    // Wait briefly before checking the results
    println!("Waiting for proofs to be generated...");
    tokio::time::sleep(Duration::from_secs(90)).await;

    // Verify the results for each port
    for port in [9000, 9001, 9002] {
        let mut attempts = 0;
        let max_attempts = 30;

        loop {
            let response = client
                .get(format!(
                    "http://localhost:{}/proof/{}",
                    port, "test_proof_id",
                ))
                .send()
                .await?;

            // 最後にProofStatusへ変換
            let status: ProofStatus = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // デバッグ用に出力
            println!("Proof Status from port {}: {:?}", port, status);

            if status.state == "completed" {
                println!("Proof generation completed successfully on port {}", port);
                break;
            }

            if status.state == "failed" {
                panic!(
                    "Proof generation failed on port {}: {:?}",
                    port, status.message
                );
            }

            attempts += 1;
            if attempts >= max_attempts {
                panic!("Timeout waiting for proof generation on port {}", port);
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

// #[tokio::test]
// async fn test_multiplication_circuit() {
//     // 同様のパターンで乗算回路のテスト
//     let input = vec![5, 4];
//     // ...同様のテストロジック
// }

// #[tokio::test]
// async fn test_comparison_circuit() {
//     // 比較回路のテスト
//     let input = vec![15, 10];
//     // ...同様のテストロジック
// }

// #[tokio::test]
// async fn test_range_proof_circuit() {
//     // 範囲証明回路のテスト
//     let input = vec![50]; // 0-100の範囲内であることの証明
//                           // ...同様のテストロジック
// }

// #[tokio::test]
// async fn test_complex_arithmetic_circuit() {
//     // 複合演算回路のテスト (例: (a + b) * c)
//     let input = vec![3, 4, 2];
//     // ...同様のテストロジック
// }
