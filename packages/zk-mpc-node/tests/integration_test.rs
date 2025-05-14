use ark_bls12_377::Fr;
use ed25519_dalek::Keypair;
use mpc_algebra::Reveal;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serial_test::serial;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;
use zk_mpc::circuits::circuit::MySimpleCircuit;
use zk_mpc::marlin::MFr;
use zk_mpc_node::{
    BuiltinCircuit, CircuitIdentifier, ProofOutput, ProofOutputType, ProofRequest, UserPublicKey,
};

#[derive(Debug, Serialize, Deserialize)]
struct ProofStatus {
    state: String,
    proof_id: String,
    message: Option<String>,
    output: Option<ProofOutput>,
}

struct TestNodes {
    processes: Vec<Child>,
}

impl TestNodes {
    async fn start(count: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let mut processes = vec![];
        for id in 0..count {
            let node = Command::new("cargo")
                .args(["run", "--release", &id.to_string(), "address/3"])
                .spawn()?;
            processes.push(node);
        }

        // wait for the nodes to start
        sleep(Duration::from_secs(15)).await;
        Ok(Self { processes })
    }
}

impl Drop for TestNodes {
    fn drop(&mut self) {
        for mut node in self.processes.drain(..) {
            let _ = node.kill();
        }
    }
}

#[tokio::test]
#[serial]
async fn test_mpc_node_proof_generation() -> Result<(), Box<dyn std::error::Error>> {
    // let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();

    let fa = MFr::from_public(Fr::from(2));
    let fb = MFr::from_public(Fr::from(3));

    let circuit = MySimpleCircuit {
        a: Some(fa),
        b: Some(fb),
    };

    // Send requests to the three ports
    for port in [9000, 9001, 9002] {
        println!("Sending request to port {}", port);
        let response = client
            .post(format!("http://localhost:{}", port))
            .json(&ProofRequest {
                proof_id: "test_proof_id".to_string(),
                circuit_type: CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit.clone())),
                output_type: ProofOutputType::Public,
            })
            .send()
            .await?;

        assert!(response.status().is_success());
        println!("Response from port {}: {:?}", port, response);

        let response_body: serde_json::Value = response.json().await?;
        assert_eq!(response_body["success"], true);
        assert!(response_body["message"]
            .as_str()
            .unwrap()
            .contains("successfully"));
    }

    // Wait briefly before checking the results
    println!("Waiting for proofs to be generated...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify the results for each port
    for port in [9000, 9001, 9002] {
        let mut attempts = 0;
        let max_attempts = 30;

        loop {
            let status = client
                .get(format!(
                    "http://localhost:{}/proof/{}",
                    port, "test_proof_id",
                ))
                .send()
                .await?
                .json::<ProofStatus>()
                .await?;

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

#[tokio::test]
#[serial]
async fn test_mpc_node_invalid_request() -> Result<(), Box<dyn std::error::Error>> {
    // let _nodes = TestNodes::start(3).await?;

    // Test invalid requests
    let client = reqwest::Client::new();

    // Test on each port
    for port in [9000, 9001, 9002] {
        println!("Testing invalid request on port {}", port);
        match client
            .post(format!("http://localhost:{}", port))
            .json(&json!({
                "invalid": "data"
            }))
            .send()
            .await
        {
            Ok(response) => {
                assert!(!response.status().is_success());
                println!(
                    "Got expected error response from port {}: {:?}",
                    port,
                    response.status()
                );
            }
            Err(e) => {
                // Allow error responses as well
                println!("Got expected error from port {}: {:?}", port, e);
            }
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mpc_node_multiple_requests() -> Result<(), Box<dyn std::error::Error>> {
    // let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();

    // List of test cases
    for (a, b) in [(2, 3), (5, 7), (11, 13)] {
        let proof_id = format!("test_proof_id_{}_{}", a, b);

        let fa = MFr::from_public(Fr::from(a));
        let fb = MFr::from_public(Fr::from(b));

        let circuit = MySimpleCircuit {
            a: Some(fa),
            b: Some(fb),
        };

        // Sequentially send requests to the three ports
        for port in [9000, 9001, 9002] {
            println!("Testing port {} with inputs a={}, b={}", port, a, b);
            let response = client
                .post(format!("http://localhost:{}", port))
                .json(&ProofRequest {
                    proof_id: proof_id.clone(),
                    circuit_type: CircuitIdentifier::Built(BuiltinCircuit::MySimple(
                        circuit.clone(),
                    )),
                    output_type: ProofOutputType::Public,
                })
                .send()
                .await?;

            assert!(response.status().is_success());
            println!(
                "Response from port {} with inputs a={}, b={}: {:?}",
                port, a, b, response
            );

            let response_body: serde_json::Value = response.json().await?;
            assert_eq!(response_body["success"], true);
            assert!(response_body["message"]
                .as_str()
                .unwrap()
                .contains("successfully"));
        }

        // Wait briefly before checking the results
        println!("Waiting for proofs to be generated...");
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Verify the results for each port
        for port in [9000, 9001, 9002] {
            let mut attempts = 0;
            let max_attempts = 30;

            loop {
                let status = client
                    .get(format!("http://localhost:{}/proof/{}", port, proof_id))
                    .send()
                    .await?
                    .json::<ProofStatus>()
                    .await?;

                println!(
                    "Proof Status from port {} with inputs a={}, b={}: {:?}",
                    port, a, b, status
                );

                if status.state == "completed" {
                    println!(
                        "Proof generation completed successfully on port {} with inputs a={}, b={}",
                        port, a, b
                    );
                    break;
                }

                if status.state == "failed" {
                    panic!(
                        "Proof generation failed on port {} with inputs a={}, b={}: {:?}",
                        port, a, b, status.message
                    );
                }

                attempts += 1;
                if attempts >= max_attempts {
                    panic!(
                        "Timeout waiting for proof generation on port {} with inputs a={}, b={}",
                        port, a, b
                    );
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        // Wait briefly before proceeding to the next test case
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mpc_node_different_outputs() -> Result<(), Box<dyn std::error::Error>> {
    // let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();

    let keypair = Keypair::generate(&mut OsRng {});

    let user_pubkeys = vec![
        UserPublicKey::new(
            "user_pubkey_1".to_string(),
            base64::encode(keypair.public.as_bytes()),
        ),
        UserPublicKey::new(
            "user_pubkey_2".to_string(),
            base64::encode(keypair.public.as_bytes()),
        ),
        UserPublicKey::new(
            "user_pubkey_3".to_string(),
            base64::encode(keypair.public.as_bytes()),
        ),
    ];

    // 異なる出力タイプのテストケース
    let test_cases = vec![
        ("public_output", ProofOutputType::Public),
        (
            "private_to_public",
            ProofOutputType::PrivateToPublic(user_pubkeys.clone()),
        ),
        (
            "private_to_private",
            ProofOutputType::PrivateToPrivate(user_pubkeys[0].public_key.clone()),
        ),
    ];

    for (test_id, output_type) in test_cases {
        let proof_id = format!("test_output_{}", test_id);
        let fa = MFr::from_public(Fr::from(2));
        let fb = MFr::from_public(Fr::from(3));

        let circuit = MySimpleCircuit {
            a: Some(fa),
            b: Some(fb),
        };

        // 3つのノードそれぞれにリクエストを送信
        for port in [9000, 9001, 9002] {
            println!("Testing port {} with output type {:?}", port, output_type);
            let response = client
                .post(format!("http://localhost:{}", port))
                .json(&ProofRequest {
                    proof_id: proof_id.clone(),
                    circuit_type: CircuitIdentifier::Built(BuiltinCircuit::MySimple(
                        circuit.clone(),
                    )),
                    output_type: output_type.clone(),
                })
                .send()
                .await?;

            assert!(response.status().is_success());
            let response_body: serde_json::Value = response.json().await?;
            assert_eq!(response_body["success"], true);
        }

        // 結果の検証を待機
        println!(
            "Waiting for proof generation with output type {:?}...",
            output_type
        );
        tokio::time::sleep(Duration::from_secs(5)).await;

        // 各ノードの結果を確認
        for port in [9000, 9001, 9002] {
            let mut attempts = 0;
            let max_attempts = 30;

            loop {
                let status = client
                    .get(format!("http://localhost:{}/proof/{}", port, proof_id))
                    .send()
                    .await?
                    .json::<ProofStatus>()
                    .await?;

                if status.state == "completed" {
                    // 出力タイプに応じた検証
                    let output = client
                        .get(format!(
                            "http://localhost:{}/proof/{}/output",
                            port, proof_id
                        ))
                        .send()
                        .await?
                        .json::<ProofOutput>()
                        .await?;

                    match output_type {
                        ProofOutputType::Public => {
                            assert!(output.value.is_some());
                            assert!(output.shares.is_none());
                        }
                        ProofOutputType::PrivateToPublic(_) => {
                            assert!(output.shares.is_some());
                            assert!(output.value.is_none());
                        }
                        ProofOutputType::PrivateToPrivate(_) => {
                            assert!(output.value.is_some());
                            assert!(output.shares.is_none());
                        }
                    }
                    break;
                }

                if status.state == "failed" {
                    panic!(
                        "Proof generation failed on port {} with output type {:?}: {:?}",
                        port, output_type, status.message
                    );
                }

                attempts += 1;
                if attempts >= max_attempts {
                    panic!(
                        "Timeout waiting for proof generation on port {} with output type {:?}",
                        port, output_type
                    );
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        // 次のテストケースに進む前に待機
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
