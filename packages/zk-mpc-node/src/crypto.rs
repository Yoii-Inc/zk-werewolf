use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Key generation failed: {0}")]
    KeyGenerationError(String),
    #[error("Encryption failed: {0}")]
    EncryptionError(String),
    #[error("Decryption failed: {0}")]
    DecryptionError(String),
    #[error("Key not initialized")]
    KeyNotInitialized,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeKeys {
    pub public_key: String, // Base64エンコードされた公開鍵
    #[serde(skip_serializing)] // シリアライズ時にスキップ
    secret_key: String, // Base64エンコードされた秘密鍵
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublicKey {
    pub user_id: String,
    pub public_key: String,
}

impl UserPublicKey {
    pub fn new(user_id: String, public_key: String) -> Self {
        Self {
            user_id,
            public_key,
        }
    }
}

pub struct KeyManager {
    keys: Arc<RwLock<Option<NodeKeys>>>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn generate_keypair(&self) -> Result<NodeKeys, CryptoError> {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);

        let keys = NodeKeys {
            public_key: base64::encode(keypair.public.as_bytes()),
            secret_key: base64::encode(keypair.secret.as_bytes()),
        };

        *self.keys.write().await = Some(keys.clone());
        Ok(keys)
    }

    pub async fn get_public_key(&self) -> Result<String, CryptoError> {
        self.keys
            .read()
            .await
            .as_ref()
            .map(|k| k.public_key.clone())
            .ok_or(CryptoError::KeyNotInitialized)
    }

    pub async fn encrypt_share(
        &self,
        share: &[u8],
        recipient_public_key: &str,
    ) -> Result<Vec<u8>, CryptoError> {
        // Base64デコード
        let public_key_bytes = base64::decode(recipient_public_key)
            .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;

        let public_key = PublicKey::from_bytes(&public_key_bytes)
            .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;

        // TODO: 実際の暗号化処理を実装
        // この例では単純にデータを返していますが、実際にはEd25519で暗号化する必要があります
        Ok(share.to_vec())
    }

    pub async fn decrypt_share(&self, encrypted_share: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let keys = self.keys.read().await;
        let node_keys = keys.as_ref().ok_or(CryptoError::KeyNotInitialized)?;

        // Base64デコード
        let secret_key_bytes = base64::decode(&node_keys.secret_key)
            .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;

        // TODO: 実際の復号処理を実装
        // この例では単純にデータを返していますが、実際にはEd25519で復号する必要があります
        Ok(encrypted_share.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_generation() {
        let key_manager = KeyManager::new();
        let keys = key_manager.generate_keypair().await.unwrap();

        assert!(!keys.public_key.is_empty());
        assert!(!keys.secret_key.is_empty());
    }

    #[tokio::test]
    async fn test_get_public_key() {
        let key_manager = KeyManager::new();
        let keys = key_manager.generate_keypair().await.unwrap();
        let public_key = key_manager.get_public_key().await.unwrap();

        assert_eq!(keys.public_key, public_key);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_share() {
        let key_manager = KeyManager::new();
        key_manager.generate_keypair().await.unwrap();
        let public_key = key_manager.get_public_key().await.unwrap();

        let test_share = b"test share data";
        let encrypted = key_manager
            .encrypt_share(test_share, &public_key)
            .await
            .unwrap();
        let decrypted = key_manager.decrypt_share(&encrypted).await.unwrap();

        assert_eq!(test_share.to_vec(), decrypted);
    }
}
