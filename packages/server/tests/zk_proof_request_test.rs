use ark_bls12_377::Fr;
use ark_ff::{PrimeField, UniformRand};
use ark_std::test_rng;
use base64::decode;
use crypto_box::{PublicKey, SecretKey};
use mpc_algebra::{CommitmentScheme, FromLocal};
use mpc_algebra_wasm::{
    types::AnonymousVotingInput, AnonymousVotingEncryption, AnonymousVotingOutput,
    CircuitEncryptedInputIdentifier, NodeKey, SecretSharingScheme, SplitAndEncrypt,
};
// use mpc_circuits::inputs::anonymous_voting::{
//     AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
// };
use serde_json::json;
use tokio_tungstenite::tungstenite::client;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
use zk_mpc::circuits::{AnonymousVotingCircuit, LocalOrMPC};
use zk_mpc_node::{NodeKeys, ProofOutputType, ProofStatus};

use ark_std::PubUniformRand;
use mpc_algebra_wasm::mpc_circuits_wasm::inputs::anonymous_voting::{
    AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
};
use server::models::game::{BatchRequest, BatchStatus, ClientRequestType};
use std::time::Duration;
use tokio::time::sleep;

const NODE_NUM: usize = 3;
const USER_NUM: usize = 3;

fn get_node_keys() -> (Vec<NodeKey>, Vec<String>) {
    // data/node_keys_0.json, data/node_keys_1.json, data/node_keys_2.jsonから読み込む
    let mut keys = Vec::new();
    let mut secret_keys = Vec::new();
    for i in 0..NODE_NUM {
        // ファイルから読み込む
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let file_path = format!("./../zk-mpc-node/{}/node_keys_{}.json", data_dir, i);
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
}

fn setup_voting_dummy() -> ClientRequestType {
    let anon_voting_output = AnonymousVotingOutput {
        shares: vec![],
        public_input: mpc_algebra_wasm::AnonymousVotingPublicInput {
            pedersen_param: <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(&mut test_rng())
                .unwrap(),
            player_commitment: vec![],
        },
    };
    ClientRequestType::AnonymousVoting(anon_voting_output)
}

#[tokio::test]
async fn test_batch_request() {
    // // BatchServiceの初期化（バッチサイズ3で設定）
    let mut batch_request = BatchRequest::new();

    // // 投票用の回路とインプットのセットアップ
    // let rng = &mut test_rng();
    // let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng).unwrap();

    // // 2つの異なる投票データを作成
    // let private_input1 = AnonymousVotingPrivateInput {
    //     id: 1,
    //     is_target_id: vec![Fr::from(1), Fr::from(0), Fr::from(0)], // 1番目のプレイヤーに投票
    //     player_randomness: Fr::rand(rng),
    // };

    // let private_input2 = AnonymousVotingPrivateInput {
    //     id: 2,
    //     is_target_id: vec![Fr::from(0), Fr::from(1), Fr::from(0)], // 2番目のプレイヤーに投票
    //     player_randomness: Fr::rand(rng),
    // };

    // let public_input = AnonymousVotingPublicInput {
    //     pedersen_param: pedersen_param.clone(),
    //     player_commitment: vec![
    //         <Fr as LocalOrMPC<Fr>>::PedersenCommitment::default();
    //         3  // プレイヤー数
    //     ],
    // };

    // let circuit1 = AnonymousVotingCircuit {
    //     is_target_id: vec![private_input1.is_target_id.clone()],
    //     pedersen_param: public_input.pedersen_param.clone(),
    //     player_randomness: vec![private_input1.player_randomness],
    //     player_commitment: public_input.player_commitment.clone(),
    // };

    // let circuit2 = AnonymousVotingCircuit {
    //     is_target_id: vec![private_input2.is_target_id.clone()],
    //     pedersen_param: public_input.pedersen_param.clone(),
    //     player_randomness: vec![private_input2.player_randomness],
    //     player_commitment: public_input.player_commitment.clone(),
    // };

    // 1つ目のリクエストを追加
    let request1 = setup_voting_dummy();
    let batch_id1 = batch_request.add_request(request1).await;

    // 2つ目のリクエストを追加
    let request2 = setup_voting_dummy();
    let batch_id2 = batch_request.add_request(request2).await;

    // 3つ目のリクエストを追加（これでバッチサイズに達する）
    let request3 = setup_voting_dummy();
    let batch_id3 = batch_request.add_request(request3).await;

    // 同じバッチIDであることを確認
    assert_eq!(batch_id1, batch_id2);
    assert_eq!(batch_id2, batch_id3);

    // バッチ処理が開始されるのを少し待つ
    sleep(Duration::from_millis(100)).await;

    // 新しいリクエストは新しいバッチIDを取得することを確認
    let request4 = setup_voting_dummy();
    let batch_id4 = batch_request.add_request(request4).await;
    assert_ne!(batch_id1, batch_id4);
}

#[tokio::test]
async fn test_real_batch_request() -> Result<(), anyhow::Error> {
    // BatchServiceの初期化（バッチサイズ3で設定）
    let mut batch_request = BatchRequest::new();

    let (circuit_encrypted_input, _user_keys) = setup_voting();

    let input_vec = match &circuit_encrypted_input {
        CircuitEncryptedInputIdentifier::AnonymousVoting(shares) => shares,
        _ => panic!("Expected AnonymousVoting input"),
    };
    println!("Number of shares: {}", input_vec.len());

    // 1つ目のリクエストを追加
    // let request1 = setup_voting_dummy();
    let request1 = ClientRequestType::AnonymousVoting(input_vec[0].clone());
    let batch_id1 = batch_request.add_request(request1).await;

    // 2つ目のリクエストを追加
    let request2 = ClientRequestType::AnonymousVoting(input_vec[1].clone());
    let batch_id2 = batch_request.add_request(request2).await;

    // 3つ目のリクエストを追加（これでバッチサイズに達する）
    let request3 = ClientRequestType::AnonymousVoting(input_vec[2].clone());
    let batch_id3 = batch_request.add_request(request3).await;

    // 同じバッチIDであることを確認
    assert_eq!(batch_id1, batch_id2);
    assert_eq!(batch_id2, batch_id3);

    // バッチ処理が開始されるのを少し待つ
    sleep(Duration::from_millis(100)).await;

    // 新しいリクエストは新しいバッチIDを取得することを確認
    let request4 = setup_voting_dummy();
    let batch_id4 = batch_request.add_request(request4).await;
    assert_ne!(batch_id1, batch_id4);

    sleep(Duration::from_secs(30)).await; // バッチ処理が完了するのを待つ

    let client = reqwest::Client::new();

    tokio::time::sleep(Duration::from_secs(10)).await;

    // Send requests to the three ports

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
                .map_err(|e| format!("Failed to parse response: {}", e))
                .unwrap();

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
