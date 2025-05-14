use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeKey {
    pub node_id: u32,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterKeyResponse {
    pub success: bool,
    pub node_id: u32,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}
