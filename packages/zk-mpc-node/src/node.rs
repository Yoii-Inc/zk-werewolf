use crate::crypto::KeyManager;
use crate::proof::ProofManager;
use crate::server::ApiClient; // 追加
use crate::{models::ProofRequest, CircuitFactory};
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
    pub key_manager: Arc<KeyManager>,
    pub api_client: Arc<ApiClient>, // 追加
}

impl<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static> Node<IO> {
    pub async fn new(
        id: u32,
        net: Arc<MPCNetConnection<IO>>,
        proof_manager: Arc<ProofManager>,
        server_url: String, // 追加
    ) -> Self {
        let key_manager = Arc::new(KeyManager::new());
        // 起動時に鍵ペアを生成
        key_manager
            .generate_keypair()
            .await
            .expect("Failed to generate keypair");

        let api_client = Arc::new(ApiClient::new(server_url.clone()));

        let node = Self {
            id,
            net,
            proof_manager,
            key_manager,
            api_client: api_client.clone(),
        };

        // 生成した公開鍵をサーバーに登録
        node.register_public_key()
            .await
            .expect("Failed to register public key with server");

        node
    }

    // 公開鍵を登録するメソッドを追加
    pub async fn register_public_key(&self) -> Result<(), Box<dyn std::error::Error>> {
        let public_key = self.key_manager.get_public_key().await?;
        self.api_client
            .register_public_key(self.id, public_key)
            .await?;
        Ok(())
    }

    pub async fn generate_proof(&self, request: ProofRequest, proof_id: String) {
        let pm = self.proof_manager.clone();

        // Setup circuit
        let local_circuit = CircuitFactory::create_local_circuit(&request);

        let mpc_circuit = CircuitFactory::create_mpc_circuit(&request);

        let inputs = CircuitFactory::create_verify_inputs(&request);

        let (index_pk, index_vk) =
            LocalMarlin::index(&self.proof_manager.srs, local_circuit).unwrap();
        let mpc_index_pk = IndexProverKey::from_public(index_pk);

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

    pub async fn get_public_key(&self) -> Result<String, crate::crypto::CryptoError> {
        self.key_manager.get_public_key().await
    }

    pub async fn encrypt_share(
        &self,
        share: &[u8],
        recipient_public_key: &str,
    ) -> Result<Vec<u8>, crate::crypto::CryptoError> {
        self.key_manager
            .encrypt_share(share, recipient_public_key)
            .await
    }

    pub async fn decrypt_share(
        &self,
        encrypted_share: &[u8],
    ) -> Result<Vec<u8>, crate::crypto::CryptoError> {
        self.key_manager.decrypt_share(encrypted_share).await
    }
}
