use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPublicKeyRequest {
    pub node_id: u32,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPublicKeyResponse {
    pub success: bool,
    pub node_id: u32,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}
