use mpc_net::multi::MPCNetConnection;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::net::TcpListener;

use zk_mpc_node::{models::Opt, node::Node, proof::ProofManager, server::handle_client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    // Initialize ProofManager
    let proof_manager = Arc::new(ProofManager::new());

    // Create a listener for client connections
    let listener = TcpListener::bind(format!("127.0.0.1:{}", 9000 + opt.id)).await?;

    // Initialize the MPC network
    let mut net = MPCNetConnection::init_from_path(&opt.input, opt.id as u32);
    net.listen().await.expect("Failed to listen");
    net.connect_to_all()
        .await
        .expect("Failed to connect to all");

    println!("Listening on port {}", 9000 + opt.id);

    // Initialize the node
    let node = Arc::new(Node::new(opt.id as u32, Arc::new(net), proof_manager.clone()).await);

    // Loop to accept requests from clients
    while let Ok((socket, _)) = listener.accept().await {
        let pm = proof_manager.clone();
        let node = node.clone();
        tokio::spawn(async move {
            handle_client(socket, pm, node).await;
        });
    }

    Ok(())
}
