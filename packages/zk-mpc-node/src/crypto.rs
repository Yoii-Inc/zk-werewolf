use base64::{decode, encode};
use crypto_box::{
    aead::{Aead, AeadCore, OsRng},
    PublicKey, SalsaBox, SecretKey,
};
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
    // #[serde(skip_serializing)] // シリアライズ時にスキップ
    pub secret_key: String, // Base64エンコードされた秘密鍵
}

#[derive(Serialize, Deserialize)]
pub struct KeyFile {
    pub public_key: String, // Base64エンコードされた公開鍵
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

    pub async fn generate_keypair(&self, id: u32) -> Result<NodeKeys, CryptoError> {
        let secret_key = SecretKey::generate(&mut OsRng);
        let public_key = PublicKey::from(&secret_key);

        let keys = NodeKeys {
            public_key: encode(public_key.to_bytes()),
            secret_key: encode(secret_key.to_bytes()),
        };

        Self::write_keys_to_file(id, keys.clone());

        *self.keys.write().await = Some(keys.clone());
        Ok(keys)
    }

    pub async fn load_keypair(&self, id: u32) -> Result<NodeKeys, CryptoError> {
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let file_path = format!("{}/node_keys_{}.json", data_dir, id);

        let file = std::fs::File::open(&file_path).map_err(|e| {
            CryptoError::KeyGenerationError(format!("Failed to open key file: {}", e))
        })?;

        let reader = std::io::BufReader::new(file);
        let keys: NodeKeys = serde_json::from_reader(reader).map_err(|e| {
            CryptoError::KeyGenerationError(format!("Failed to parse key file: {}", e))
        })?;

        *self.keys.write().await = Some(keys.clone());
        Ok(keys)
    }

    pub fn write_keys_to_file(id: u32, keys: crate::NodeKeys) {
        // NodeKeysをjsonにシリアライズしてファイルに保存する
        let keys_json = serde_json::to_string(&keys).expect("Failed to serialize keys");
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        // ディレクトリが存在しない場合は作成
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
        let file_path = format!("{}/node_keys_{}.json", data_dir, id);
        std::fs::write(file_path, keys_json).expect("Failed to write keys to file");
    }

    pub async fn get_public_key(&self) -> Result<String, CryptoError> {
        self.keys
            .read()
            .await
            .as_ref()
            .map(|k| k.public_key.clone())
            .ok_or(CryptoError::KeyNotInitialized)
    }

    pub async fn get_secret_key(&self) -> Result<String, CryptoError> {
        self.keys
            .read()
            .await
            .as_ref()
            .map(|k| k.secret_key.clone())
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
        let secret_key = SecretKey::from_slice(&secret_key_bytes)
            .map_err(|_| CryptoError::EncryptionError("Invalid secret key".to_string()))?;

        // 受信者の公開鍵をBase64デコード
        let recipient_key_bytes = decode(recipient_public_key)
            .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;
        let recipient_key = PublicKey::from_slice(&recipient_key_bytes).map_err(|_| {
            CryptoError::EncryptionError("Invalid recipient public key".to_string())
        })?;

        // 暗号化
        let salsa_box = SalsaBox::new(&recipient_key, &secret_key);
        let nonce = SalsaBox::generate_nonce(&mut OsRng);
        let encrypted = salsa_box
            .encrypt(&nonce, share)
            .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;

        // ノンスと暗号文を結合
        let mut result = Vec::new();
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    pub async fn decrypt_share(&self, encrypted_share: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let keys = self.keys.read().await;
        let node_keys = keys.as_ref().ok_or(CryptoError::KeyNotInitialized)?;

        // 秘密鍵をBase64デコード
        let secret_key_bytes = decode(&node_keys.secret_key)
            .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;
        let secret_key = SecretKey::from_slice(&secret_key_bytes)
            .map_err(|_| CryptoError::DecryptionError("Invalid secret key".to_string()))?;

        // 公開鍵をBase64デコード
        let public_key_bytes = decode(&node_keys.public_key)
            .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;
        let public_key = PublicKey::from_slice(&public_key_bytes)
            .map_err(|_| CryptoError::DecryptionError("Invalid public key".to_string()))?;

        if encrypted_share.len() < 24 {
            return Err(CryptoError::DecryptionError(
                "Invalid encrypted data length".to_string(),
            ));
        }

        // ノンスと暗号文を分離
        let (nonce_bytes, cipher_text) = encrypted_share.split_at(24);
        let nonce = *crypto_box::Nonce::from_slice(nonce_bytes);

        // 復号化
        let salsa_box = SalsaBox::new(&public_key, &secret_key);
        salsa_box
            .decrypt(&nonce, cipher_text)
            .map_err(|e| CryptoError::DecryptionError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_generation() {
        let key_manager = KeyManager::new();
        let keys = key_manager.generate_keypair(1).await.unwrap();

        assert!(!keys.public_key.is_empty());
        assert!(!keys.secret_key.is_empty());
    }

    #[tokio::test]
    async fn test_get_public_key() {
        let key_manager = KeyManager::new();
        let keys = key_manager.generate_keypair(1).await.unwrap();
        let public_key = key_manager.get_public_key().await.unwrap();

        assert_eq!(keys.public_key, public_key);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_share() {
        let key_manager = KeyManager::new();
        key_manager.generate_keypair(1).await.unwrap();
        let public_key = key_manager.get_public_key().await.unwrap();

        let test_share = b"test share data";
        let encrypted = key_manager
            .encrypt_share(test_share, &public_key)
            .await
            .unwrap();
        let decrypted = key_manager.decrypt_share(&encrypted).await.unwrap();

        assert_eq!(test_share.to_vec(), decrypted);
    }

    #[tokio::test]
    async fn test_write_and_read_keys() {
        let key_manager = KeyManager::new();
        let keys = key_manager.generate_keypair(1).await.unwrap();

        KeyManager::write_keys_to_file(1, keys.clone());

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let file_path = format!("{}/node_keys_{}.json", data_dir, 1);
        let file = std::fs::File::open(file_path).unwrap();
        let reader = std::io::BufReader::new(file);
        let key_file: KeyFile = serde_json::from_reader(reader).unwrap();

        assert_eq!(keys.public_key, key_file.public_key);
    }
}
