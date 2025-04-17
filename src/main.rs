use mpc_net::multi::MPCNetConnection;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::net::TcpListener;

use zk_mpc_node::{models::Opt, node::Node, proof::ProofManager, server::handle_client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    // ProofManagerの初期化
    let proof_manager = Arc::new(ProofManager::new());

    // クライアント接続用のリスナーを作成
    let listener = TcpListener::bind(format!("127.0.0.1:{}", 9000 + opt.id)).await?;

    // MPCネットワークの初期化
    let mut net = MPCNetConnection::init_from_path(&opt.input, opt.id as u32);
    net.listen().await.expect("Failed to listen");
    net.connect_to_all()
        .await
        .expect("Failed to connect to all");

    println!("Listening on port {}", 9000 + opt.id);

    // ノードの初期化
    let node = Arc::new(Node::new(opt.id as u32, Arc::new(net), proof_manager.clone()).await);

    // クライアントからのリクエストを受け付けるループ
    while let Ok((socket, _)) = listener.accept().await {
        let pm = proof_manager.clone();
        let node = node.clone();
        tokio::spawn(async move {
            handle_client(socket, pm, node).await;
        });
    }

    Ok(())
}
