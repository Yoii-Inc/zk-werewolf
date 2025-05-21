use sodiumoxide::crypto::box_;
use base64::{encode, decode};
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
        // sodiumoxideを初期化
        sodiumoxide::init().expect("Failed to initialize sodiumoxide");
        
        Self {
            keys: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn generate_keypair(&self) -> Result<NodeKeys, CryptoError> {
        let (public_key, secret_key) = box_::gen_keypair();

        let keys = NodeKeys {
            public_key: encode(public_key.as_ref()),
            secret_key: encode(secret_key.as_ref()),
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
        let keys = self.keys.read().await;
        let node_keys = keys.as_ref().ok_or(CryptoError::KeyNotInitialized)?;

        // 秘密鍵とBase64デコード
        let secret_key_bytes = decode(&node_keys.secret_key)
            .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;
        let secret_key = box_::SecretKey::from_slice(&secret_key_bytes)
            .ok_or_else(|| CryptoError::EncryptionError("Invalid secret key".to_string()))?;

        // 受信者の公開鍵をBase64デコード
        let recipient_key_bytes = decode(recipient_public_key)
            .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;
        let recipient_key = box_::PublicKey::from_slice(&recipient_key_bytes)
            .ok_or_else(|| CryptoError::EncryptionError("Invalid recipient public key".to_string()))?;

        // ノンスを生成
        let nonce = box_::gen_nonce();
        
        // 暗号化
        let encrypted = box_::seal(
            share,
            &nonce,
            &recipient_key,
            &secret_key,
        );

        // ノンスと暗号文を結合して返す
        let mut result = Vec::new();
        result.extend_from_slice(nonce.as_ref());
        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    pub async fn decrypt_share(&self, encrypted_share: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let keys = self.keys.read().await;
        let node_keys = keys.as_ref().ok_or(CryptoError::KeyNotInitialized)?;

        // 秘密鍵をBase64デコード
        let secret_key_bytes = decode(&node_keys.secret_key)
            .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;
        let secret_key = box_::SecretKey::from_slice(&secret_key_bytes)
            .ok_or_else(|| CryptoError::DecryptionError("Invalid secret key".to_string()))?;

        // ノンスと暗号文を分離
        if encrypted_share.len() < box_::NONCEBYTES {
            return Err(CryptoError::DecryptionError("Invalid encrypted data length".to_string()));
        }

        let (nonce_bytes, cipher_text) = encrypted_share.split_at(box_::NONCEBYTES);
        let nonce = box_::Nonce::from_slice(nonce_bytes)
            .ok_or_else(|| CryptoError::DecryptionError("Invalid nonce".to_string()))?;

        // 復号
        box_::open(
            cipher_text,
            &nonce,
            &box_::PublicKey::from_slice(&decode(&node_keys.public_key).map_err(|e| CryptoError::DecryptionError(e.to_string()))?)
                .ok_or_else(|| CryptoError::DecryptionError("Invalid public key".to_string()))?,
            &secret_key,
        )
        .map_err(|_| CryptoError::DecryptionError("Decryption failed".to_string()))
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
