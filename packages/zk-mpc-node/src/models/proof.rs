use mpc_algebra_wasm::CircuitEncryptedInputIdentifier;
use serde::{Deserialize, Serialize};

use crate::UserPublicKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofStatus {
    pub state: String,
    pub proof_id: String,
    pub message: Option<String>,
    pub output: Option<ProofOutput>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub proof_id: String,
    pub circuit_type: CircuitEncryptedInputIdentifier,
    pub output_type: ProofOutputType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofOutputType {
    Public,                              // 出力が公開
    PrivateToPublic(Vec<UserPublicKey>), // 出力が非公開、出力先が公開 (公開鍵をユーザー数だけ含む)
    PrivateToPrivate(String),            // 出力が非公開、出力先も非公開 (出力先の公開鍵を1つ含む)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOutput {
    pub output_type: ProofOutputType,
    pub value: Option<Vec<u8>>, // 公開値または暗号化された値
    // pub shares: Option<Vec<Vec<u8>>>, // 暗号化されたシェア
    pub shares: Option<Vec<EncryptedShare>>, // 暗号化されたシェア
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedShare {
    pub node_id: u32,            // どのノードが暗号化したか
    pub user_id: String,         // どのユーザー向けか
    pub encrypted_data: Vec<u8>, // 暗号化されたシェア
}
