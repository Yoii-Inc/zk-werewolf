use mpc_net::multi::MPCNetConnection;
use std::{env, net::SocketAddr, sync::Arc};
use structopt::StructOpt;
use zk_mpc_node::{
    models::Command, node::Node, proof::ProofManager, run_server, AppState, KeyManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic hook to ensure process termination on critical errors
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Fatal panic occurred: {}", panic_info);
        eprintln!("Location: {:?}", panic_info.location());

        // Ensure process termination
        std::process::exit(1);
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
            // 環境変数からサーバーURLを取得
            let server_url =
                env::var("SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

            println!("Using server URL: {}", server_url);

            // 環境変数からMPCアドレスを取得
            let addresses: Vec<String> = vec![
                env::var("ZK_MPC_NODE_0_TCP").unwrap_or_else(|_| "localhost:8000".to_string()),
                env::var("ZK_MPC_NODE_1_TCP").unwrap_or_else(|_| "localhost:8001".to_string()),
                env::var("ZK_MPC_NODE_2_TCP").unwrap_or_else(|_| "localhost:8002".to_string()),
            ];

            println!("Using MPC addresses: {:?}", addresses);

            // Initialize ProofManager
            let proof_manager = Arc::new(ProofManager::new());

            // Initialize the MPC network from environment addresses
            let mut net = MPCNetConnection::new(id, addresses).unwrap();
            net.listen().await.expect("Failed to listen");
            net.connect_to_all()
                .await
                .expect("Failed to connect to all");
            let key_manager = Arc::new(KeyManager::new());

            // Initialize the node
            let node = Arc::new(
                Node::new(
                    id,
                    Arc::new(net),
                    proof_manager.clone(),
                    key_manager,
                    server_url,
                )
                .await,
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

            println!("Listening on port {}", http_port);

            run_server(&addr, state).await?;

            Ok(())
        }
    }
}
