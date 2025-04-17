use serde::{Deserialize, Serialize};
use serde_json::json;
use serial_test::serial;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct ProofStatus {
    state: String,
    proof_id: String,
    message: Option<String>,
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

        // ノードの起動を待つ
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
    let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();
    let mut proof_ids = Vec::new();

    // まず3つのポートにリクエストを送信
    for port in [9000, 9001, 9002] {
        println!("Sending request to port {}", port);
        let response = client
            .post(format!("http://localhost:{}", port))
            .json(&json!({
                "circuit_type": {
                    "Built": "MySimpleCircuit"
                },
                "inputs": {
                    "Built": {
                        "MySimpleCircuit": {
                            "a": 2,
                            "b": 3
                        }
                    }
                }
            }))
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

        let proof_id = response_body["proof_id"].as_str().unwrap().to_string();
        println!("Proof ID from port {}: {}", port, proof_id);
        proof_ids.push((port, proof_id));
    }

    // 少し待ってから結果を確認
    println!("Waiting for proofs to be generated...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 各ポートの結果を確認
    for (port, proof_id) in proof_ids {
        let mut attempts = 0;
        let max_attempts = 30;

        loop {
            let status = client
                .get(format!("http://localhost:{}/proof/{}", port, proof_id))
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
    let _nodes = TestNodes::start(3).await?;

    // サーバーの起動を確実に待つ
    tokio::time::sleep(Duration::from_secs(20)).await;

    // 不正なリクエストのテスト
    let client = reqwest::Client::new();

    // 各ポートでテスト
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
                // エラーレスポンスも許容する
                println!("Got expected error from port {}: {:?}", port, e);
            }
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mpc_node_multiple_requests() -> Result<(), Box<dyn std::error::Error>> {
    let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();

    // テストケースのリスト
    for (a, b) in [(2, 3), (5, 7), (11, 13)] {
        let mut proof_ids = Vec::new();

        // 3つのポートに順次リクエストを送信
        for port in [9000, 9001, 9002] {
            println!("Testing port {} with inputs a={}, b={}", port, a, b);
            let response = client
                .post(format!("http://localhost:{}", port))
                .json(&json!({
                    "circuit_type": {
                        "Built": "MySimpleCircuit"
                    },
                    "inputs": {
                        "Built": {
                            "MySimpleCircuit": {
                                "a": a,
                                "b": b
                            }
                        }
                    }
                }))
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

            let proof_id = response_body["proof_id"].as_str().unwrap().to_string();
            proof_ids.push((port, proof_id));
        }

        // 少し待ってから結果を確認
        println!("Waiting for proofs to be generated...");
        tokio::time::sleep(Duration::from_secs(5)).await;

        // 各ポートの結果を確認
        for (port, proof_id) in proof_ids {
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

        // 次のテストケースの前に少し待機
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
