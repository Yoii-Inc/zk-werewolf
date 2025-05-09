use crate::models::ProofRequest;
use crate::proof::ProofManager;
use ark_marlin::IndexProverKey;
use mpc_algebra::Reveal;
use mpc_net::multi::MPCNetConnection;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use zk_mpc::marlin::{prove_and_verify, LocalMarlin};

pub struct Node<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static> {
    pub id: u32,
    pub net: Arc<MPCNetConnection<IO>>,
    pub proof_manager: Arc<ProofManager>,
}

impl<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static> Node<IO> {
    pub async fn new(
        id: u32,
        net: Arc<MPCNetConnection<IO>>,
        proof_manager: Arc<ProofManager>,
    ) -> Self {
        Self {
            id,
            net,
            proof_manager,
        }
    }

    pub async fn generate_proof(&self, request: ProofRequest, proof_id: String) {
        let pm = self.proof_manager.clone();

        // 回路のセットアップ
        let local_circuit = request.create_local_circuit();

        let mpc_circuit = request.create_mpc_circuit();

        let inputs = request.create_verify_inputs();

        let (index_pk, index_vk) =
            LocalMarlin::index(&self.proof_manager.srs, local_circuit).unwrap();
        let mpc_index_pk = IndexProverKey::from_public(index_pk);

        // 証明の生成と検証
        match prove_and_verify(&mpc_index_pk, &index_vk, mpc_circuit, inputs).await {
            true => {
                pm.update_proof_status(
                    &proof_id,
                    "completed",
                    Some("Proof generated successfully".to_string()),
                )
                .await;
            }
            false => {
                pm.update_proof_status(
                    &proof_id,
                    "failed",
                    Some("Proof verification failed".to_string()),
                )
                .await;
            }
        }
    }
}
