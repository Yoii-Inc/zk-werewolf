use ark_bls12_377::Fr;
use ark_ff::{PrimeField, UniformRand};
use ark_std::test_rng;
use base64::decode;
use crypto_box::{PublicKey, SecretKey};
use dotenvy::dotenv;
use mpc_algebra::{CommitmentScheme, FromLocal};
use mpc_algebra_wasm::{
    types::AnonymousVotingInput, AnonymousVotingEncryption, AnonymousVotingOutput,
    CircuitEncryptedInputIdentifier, NodeKey, SecretSharingScheme, SplitAndEncrypt,
};
// use mpc_circuits::inputs::anonymous_voting::{
//     AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
// };
use serde_json::json;
use server::{
    models::{
        game::{Game, ProverInfo},
        player::Player,
        role::Role,
        room::Room,
    },
    services::game_service,
    state::AppState,
};
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
        player_num: USER_NUM,
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
            player_num: USER_NUM,
        },
    };
    let prover_info = ProverInfo {
        user_id: "0".to_string(),
        prover_count: USER_NUM,
        encrypted_data: serde_json::to_string(&anon_voting_output).unwrap(),
        public_key: None,
    };
    ClientRequestType::AnonymousVoting(prover_info)
}
async fn setup_test_room_with_players(state: &AppState) -> String {
    let room_id = "test_room".to_string();
    // プレイヤーを4人作成（村人2人、占い師1人、人狼1人）
    let players = vec![
        Player {
            id: "1".to_string(),
            name: "Player1".to_string(),
            // role: Some(Role::Villager),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "2".to_string(),
            name: "Player2".to_string(),
            // role: Some(Role::Seer),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "3".to_string(),
            name: "Player3".to_string(),
            // role: Some(Role::Werewolf),
            is_dead: false,
            is_ready: false,
        },
        Player {
            id: "4".to_string(),
            name: "Player4".to_string(),
            // role: Some(Role::Villager),
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
async fn test_batch_request() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let state = AppState::new();
    let room_id = setup_test_room_with_players(&state).await;

    // ゲーム開始
    println!("Starting game in room: {}", room_id);
    let start_result = game_service::start_game(state.clone(), &room_id).await;
    let mut games = state.games.lock().await;
    let game = games.get_mut(&room_id).unwrap();

    let (circuit_encrypted_input, _user_keys) = setup_voting();

    let input_vec = match &circuit_encrypted_input {
        CircuitEncryptedInputIdentifier::AnonymousVoting(shares) => shares,
        _ => panic!("Expected AnonymousVoting input"),
    };
    println!("Number of shares: {}", input_vec.len());

    // 1つ目のリクエストを追加
    // let request1 = setup_voting_dummy();
    let prover_info_1 = ProverInfo {
        user_id: "0".to_string(),
        prover_count: USER_NUM,
        encrypted_data: serde_json::to_string(&input_vec[0]).unwrap(),
        public_key: None,
    };
    let request1 = ClientRequestType::AnonymousVoting(prover_info_1);
    let batch_id1 = game.add_request(request1, &state).await;

    // 2つ目のリクエストを追加
    let prover_info_2 = ProverInfo {
        user_id: "1".to_string(),
        prover_count: USER_NUM,
        encrypted_data: serde_json::to_string(&input_vec[1]).unwrap(),
        public_key: None,
    };
    let request2 = ClientRequestType::AnonymousVoting(prover_info_2);
    let batch_id2 = game.add_request(request2, &state).await;

    // 3つ目のリクエストを追加（これでバッチサイズに達する）
    let prover_info_3 = ProverInfo {
        user_id: "2".to_string(),
        prover_count: USER_NUM,
        encrypted_data: serde_json::to_string(&input_vec[2]).unwrap(),
        public_key: None,
    };
    let request3 = ClientRequestType::AnonymousVoting(prover_info_3);
    let batch_id3 = game.add_request(request3, &state).await;

    // 同じバッチIDであることを確認
    assert_eq!(batch_id1, batch_id2);
    assert_eq!(batch_id2, batch_id3);

    // バッチ処理が開始されるのを少し待つ
    sleep(Duration::from_millis(100)).await;

    // 新しいリクエストは新しいバッチIDを取得することを確認
    let request4 = setup_voting_dummy();
    let batch_id4 = game.add_request(request4, &state).await;
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
                .get(format!("http://localhost:{}/proof/{}", port, batch_id1,))
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
