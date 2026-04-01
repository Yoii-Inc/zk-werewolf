use mpc_net::multi::MPCNetConnection;
use std::{env, net::SocketAddr, sync::Arc, time::Duration, time::Instant};
use structopt::StructOpt;
use tokio::time::sleep;
use zk_mpc_node::{
    models::Command, node::Node, proof::ProofManager, run_server, AppState, KeyManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log panic details; individual panics are handled per-request in server code.
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Fatal panic occurred: {}", panic_info);
        eprintln!("Location: {:?}", panic_info.location());
    }));

    let command = Command::from_args();

    match command {
        Command::KeyGen { id } => {
            println!("Generating keypair for node {}", id);
            let key_manager = KeyManager::new();
            let keys = key_manager.generate_keypair(id, None).await?;
            println!("Keypair generated and saved successfully");
            println!("Public key: {}", keys.public_key);
            Ok(())
        }
        Command::Start { id } => {
            let boot_started = Instant::now();
            println!("[node:boot] start command received for node id={id}");

            // 環境変数からサーバーURLを取得
            let server_url =
                env::var("SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

            println!("[node:boot] using server URL: {}", server_url);

            // 環境変数からMPCアドレスを取得
            let addresses: Vec<String> = vec![
                env::var("ZK_MPC_NODE_0_TCP").unwrap_or_else(|_| "localhost:8000".to_string()),
                env::var("ZK_MPC_NODE_1_TCP").unwrap_or_else(|_| "localhost:8001".to_string()),
                env::var("ZK_MPC_NODE_2_TCP").unwrap_or_else(|_| "localhost:8002".to_string()),
            ];

            println!("[node:boot] using MPC addresses: {:?}", addresses);

            // Initialize ProofManager
            let proof_manager = Arc::new(ProofManager::new());

            // Initialize the MPC network from environment addresses
            println!("[node:boot] creating MPC network connection for node {id}");
            let mut net = MPCNetConnection::new(id, addresses).unwrap();
            println!("[node:boot] start listening for MPC peers...");
            net.listen().await.expect("Failed to listen");
            println!("[node:boot] MPC listener is ready");

            let connect_retry_max = env::var("MPC_CONNECT_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(30);
            let connect_retry_interval_ms = env::var("MPC_CONNECT_RETRY_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(20000);
            let connect_retry_interval = Duration::from_millis(connect_retry_interval_ms);
            let key_manager = Arc::new(KeyManager::new());

            // Initialize the node
            let node_init_started = Instant::now();
            println!("[node:boot] start Node::new for node {id}...");
            let mut node = Node::new(id, net, proof_manager.clone(), key_manager, server_url).await;
            println!(
                "[node:boot] finished Node::new for node {id} in {} ms",
                node_init_started.elapsed().as_millis()
            );

            let net = Arc::get_mut(&mut node.net).expect(
                "node.net should be uniquely owned before wrapping Node in Arc; cannot run connect_to_all",
            );
            let mut connect_attempt = 0u32;
            let connect_started = Instant::now();
            loop {
                connect_attempt += 1;
                println!(
                    "[node:boot] connecting to all peers (attempt {}/{}) after node init...",
                    connect_attempt, connect_retry_max
                );
                match net.connect_to_all().await {
                    Ok(_) => {
                        println!(
                            "[node:boot] connected to all peers (attempt {}/{}, elapsed={} ms)",
                            connect_attempt,
                            connect_retry_max,
                            connect_started.elapsed().as_millis()
                        );
                        break;
                    }
                    Err(e) if connect_attempt < connect_retry_max => {
                        eprintln!(
                            "[node:boot] failed to connect to all peers (attempt {}/{}): {:?}. Retrying in {:?}...",
                            connect_attempt, connect_retry_max, e, connect_retry_interval
                        );
                        sleep(connect_retry_interval).await;
                    }
                    Err(e) => {
                        return Err(std::io::Error::other(format!(
                            "Failed to connect to all peers after {} attempts: {:?}",
                            connect_attempt, e
                        ))
                        .into());
                    }
                }
            }
            let node = Arc::new(node);
            println!(
                "[node:boot] wrapped initialized node {id} into shared Arc after successful peer connection"
            );

            let state = AppState {
                proof_manager: proof_manager.clone(),
                node: node.clone(),
            };

            // Create a listener for client connections
            let http_port_base = env::var("MPC_HTTP_PORT")
                .unwrap_or_else(|_| "9000".to_string())
                .parse::<u16>()
                .unwrap_or(9000);

            // Each node listens on a different port based on its ID
            // e.g., node 0 -> 9000, node 1 -> 9001, node 2 -> 9002
            let http_port = http_port_base + id as u16;
            let addr = SocketAddr::from(([0, 0, 0, 0], http_port));

            println!(
                "[node:ready] node {} is ready for HTTP traffic on 0.0.0.0:{} (boot_elapsed={} ms)",
                id,
                http_port,
                boot_started.elapsed().as_millis()
            );

            run_server(&addr, state).await?;

            Ok(())
        }
    }
}
