use mpc_net::multi::MPCNetConnection;
use std::{net::SocketAddr, sync::Arc};
use structopt::StructOpt;
use zk_mpc_node::{models::Opt, node::Node, proof::ProofManager, run_server, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    // Initialize ProofManager
    let proof_manager = Arc::new(ProofManager::new());

    // Initialize the MPC network
    let mut net = MPCNetConnection::init_from_path(&opt.input, opt.id as u32);
    net.listen().await.expect("Failed to listen");
    net.connect_to_all()
        .await
        .expect("Failed to connect to all");

    let server_url = "http://localhost:8080".to_string();

    // Initialize the node
    let node = Arc::new(
        Node::new(
            opt.id as u32,
            Arc::new(net),
            proof_manager.clone(),
            server_url,
        )
        .await,
    );

    let state = AppState {
        proof_manager: proof_manager.clone(),
        node: node.clone(),
    };

    // Create a listener for client connections
    let addr = SocketAddr::from(([127, 0, 0, 1], (9000 + opt.id) as u16));

    println!("Listening on port {}", 9000 + opt.id);

    run_server(&addr, state).await?;

    Ok(())
}
