use ark_bls12_377::Fr;
use mpc_algebra::Reveal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serial_test::serial;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;
use zk_mpc::circuits::circuit::MySimpleCircuit;
use zk_mpc::marlin::MFr;
use zk_mpc_node::{BuiltinCircuit, CircuitIdentifier, ProofRequest};

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
    let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();
    let mut proof_ids = Vec::new();

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
                circuit_type: CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit.clone())),
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

        let proof_id = response_body["proof_id"].as_str().unwrap().to_string();
        println!("Proof ID from port {}: {}", port, proof_id);
        proof_ids.push((port, proof_id));
    }

    // Wait briefly before checking the results
    println!("Waiting for proofs to be generated...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify the results for each port
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

    // Ensure the server has started
    tokio::time::sleep(Duration::from_secs(20)).await;

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
    let _nodes = TestNodes::start(3).await?;
    let client = reqwest::Client::new();

    // List of test cases
    for (a, b) in [(2, 3), (5, 7), (11, 13)] {
        let mut proof_ids = Vec::new();

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
                    circuit_type: CircuitIdentifier::Built(BuiltinCircuit::MySimple(
                        circuit.clone(),
                    )),
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

            let proof_id = response_body["proof_id"].as_str().unwrap().to_string();
            proof_ids.push((port, proof_id));
        }

        // Wait briefly before checking the results
        println!("Waiting for proofs to be generated...");
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Verify the results for each port
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

        // Wait briefly before proceeding to the next test case
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
