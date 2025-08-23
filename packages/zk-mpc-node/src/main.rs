use mpc_net::multi::MPCNetConnection;
use std::{net::SocketAddr, sync::Arc};
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
        Command::Start { id, input } => {
            // Initialize ProofManager
            let proof_manager = Arc::new(ProofManager::new());

            // Initialize the MPC network
            let mut net = MPCNetConnection::init_from_path(&input, id);
            net.listen().await.expect("Failed to listen");
            net.connect_to_all()
                .await
                .expect("Failed to connect to all");

            let server_url = "http://localhost:8080".to_string();
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
            let addr = SocketAddr::from(([127, 0, 0, 1], (9000 + id) as u16));

            println!("Listening on port {}", 9000 + id);

            run_server(&addr, state).await?;

            Ok(())
        }
    }
}
