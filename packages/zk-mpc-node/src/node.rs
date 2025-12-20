use crate::crypto::KeyManager;
use crate::proof::ProofManager;
use crate::server::ApiClient;
use crate::{EncryptedShare, ProofOutput, ProofOutputType, UserPublicKey};
// 追加
use crate::models::ProofRequest;
use ark_marlin::IndexProverKey;
use mpc_algebra::Reveal;
use mpc_circuits::CircuitFactory;
use mpc_net::multi::MPCNetConnection;
use std::iter::zip;
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
        key_manager: Arc<KeyManager>,
        server_url: String, // 追加
    ) -> Self {
        // 環境変数から秘密鍵と公開鍵を取得（優先）、なければファイルから読込
        if let Ok(private_key_base64) = std::env::var("MPC_PRIVATE_KEY") {
            // 本番環境：環境変数から取得
            let public_key_env_name = format!("MPC_NODE_{}_PUBLIC_KEY", id);
            if let Ok(public_key_base64) = std::env::var(&public_key_env_name) {
                // Base64デコード
                let private_key_bytes = base64::decode(&private_key_base64)
                    .expect("Failed to decode MPC_PRIVATE_KEY from base64");
                let public_key_bytes = base64::decode(&public_key_base64)
                    .expect(&format!("Failed to decode {} from base64", public_key_env_name));

                key_manager
                    .set_keys_from_base64_bytes(private_key_bytes, public_key_bytes)
                    .await
                    .expect("Failed to set keys from environment variables");
            } else {
                panic!(
                    "Environment variable {} not found. Please set both MPC_PRIVATE_KEY and {}",
                    public_key_env_name, public_key_env_name
                );
            }
        } else {
            // 開発環境：ファイルから読込
            key_manager
                .load_keypair(id)
                .await
                .expect("Failed to load keypair from file");
        }

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

    pub async fn generate_proof(&self, request: ProofRequest) {
        let pm = self.proof_manager.clone();

        // Setup circuit
        let local_circuit = CircuitFactory::create_local_circuit(&request.circuit_type);

        let secret_key = self.key_manager.get_secret_key().await.unwrap();

        let mpc_circuit = CircuitFactory::create_mpc_circuit(
            &request.circuit_type,
            &self.id.to_string(),
            &secret_key,
        );

        let inputs = CircuitFactory::create_verify_inputs(&mpc_circuit);

        let (index_pk, index_vk) =
            LocalMarlin::index(&self.proof_manager.srs, local_circuit).unwrap();
        let mpc_index_pk = IndexProverKey::from_public(index_pk);

        let (outputs, is_valid) =
            match prove_and_verify(&mpc_index_pk, &index_vk, mpc_circuit.clone(), inputs).await {
                true => {
                    let proof_outputs = CircuitFactory::get_circuit_outputs(&mpc_circuit);
                    match &request.output_type {
                        ProofOutputType::Public => {
                            let proof_output = ProofOutput {
                                output_type: request.output_type.clone(),
                                value: Some(proof_outputs),
                                shares: None,
                            };
                            (Some(proof_output), true)
                        }
                        ProofOutputType::PrivateToPublic(pubkeys) => {
                            // TODO: 出力をシェアに分割して暗号化
                            let shares = self
                                .split_and_encrypt_output(&proof_outputs, pubkeys)
                                .await
                                .unwrap();
                            let proof_output = ProofOutput {
                                output_type: request.output_type.clone(),
                                value: None,
                                shares: Some(shares),
                            };
                            (Some(proof_output), true)
                        }
                        ProofOutputType::PrivateToPrivate(pubkey) => {
                            // TODO: 出力を直接暗号化
                            let encrypted =
                                self.encrypt_output(&proof_outputs, pubkey).await.unwrap();
                            let proof_output = ProofOutput {
                                output_type: request.output_type.clone(),
                                value: Some(encrypted),
                                shares: None,
                            };
                            (Some(proof_output), true)
                        }
                    }
                }
                false => (None, false),
            };

        println!(
            "output is {:?}",
            CircuitFactory::get_circuit_outputs(&mpc_circuit)
        );

        if is_valid {
            pm.update_proof_status_with_output(
                &request.proof_id,
                "completed",
                Some("Proof generated successfully".to_string()),
                outputs,
            )
            .await;
        } else {
            pm.update_proof_status(
                &request.proof_id,
                "failed",
                Some("Proof verification failed".to_string()),
            )
            .await;
        }
    }

    async fn split_and_encrypt_output(
        &self,
        output: &[u8],
        pubkeys: &[UserPublicKey],
    ) -> Result<Vec<EncryptedShare>, Box<dyn std::error::Error>> {
        // 出力をシェアに分割（実際の分割ロジックは省略）
        let shares = vec![output.to_vec(); pubkeys.len()]; // TODO: 実際のシェア分割を実装
        let pubkeys = pubkeys.to_vec();

        // 各シェアを暗号化
        let mut encrypted_shares = Vec::new();
        for (share, pubkey) in zip(shares, pubkeys) {
            let encrypted = self
                .key_manager
                .encrypt_share(&share, &pubkey.public_key)
                .await?;
            encrypted_shares.push(EncryptedShare {
                node_id: self.id,
                user_id: pubkey.user_id,
                encrypted_data: encrypted,
            });
        }

        Ok(encrypted_shares)
    }

    async fn encrypt_output(
        &self,
        output: &[u8],
        pubkey: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.key_manager
            .encrypt_share(output, pubkey)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
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
