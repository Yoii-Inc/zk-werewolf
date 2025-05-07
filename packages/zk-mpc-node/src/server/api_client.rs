use crate::models::{ErrorResponse, RegisterPublicKeyRequest, RegisterPublicKeyResponse};
use reqwest::Client;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Invalid node ID: {0}")]
    InvalidNodeId(u32),
    #[error("Server error: {0}")]
    ServerError(String),
}

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    pub async fn register_public_key(
        &self,
        node_id: u32,
        public_key: String,
    ) -> Result<RegisterPublicKeyResponse, ApiError> {
        // ノードIDのバリデーション
        if !(0..3).contains(&node_id) {
            return Err(ApiError::InvalidNodeId(node_id));
        }

        let request = RegisterPublicKeyRequest {
            node_id,
            public_key,
        };

        let response = self
            .client
            .post(&format!("{}/api/nodes/keys", self.base_url))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let success_response = response.json::<RegisterPublicKeyResponse>().await?;
            Ok(success_response)
        } else {
            let error_response = response.json::<ErrorResponse>().await?;
            Err(ApiError::ServerError(error_response.error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_register_public_key_success() {
        // モックサーバーの起動
        let mock_server = MockServer::start().await;

        // リクエストのマッチャーを設定
        Mock::given(method("POST"))
            .and(path("/api/nodes/keys"))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({
                "node_id": 1,
                "public_key": "test-key"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "node_id": 1,
                "public_key": "test-key"
            })))
            .mount(&mock_server)
            .await;

        let client = ApiClient::new(mock_server.uri());
        let result = client.register_public_key(1, "test-key".to_string()).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.node_id, 1);
        assert_eq!(response.public_key, "test-key");
    }

    #[tokio::test]
    async fn test_register_public_key_invalid_node_id() {
        let client = ApiClient::new("http://localhost".to_string());
        let result = client.register_public_key(4, "test-key".to_string()).await;

        assert!(matches!(result, Err(ApiError::InvalidNodeId(4))));
    }

    #[tokio::test]
    async fn test_register_public_key_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/nodes/keys"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "success": false,
                "error": "Invalid request"
            })))
            .mount(&mock_server)
            .await;

        let client = ApiClient::new(mock_server.uri());
        let result = client.register_public_key(1, "test-key".to_string()).await;

        assert!(matches!(result, Err(ApiError::ServerError(_))));
    }

    #[tokio::test]
    async fn test_register_public_key_timeout() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/nodes/keys"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(11))) // タイムアウトより長い遅延
            .mount(&mock_server)
            .await;

        let client = ApiClient::new(mock_server.uri());
        let result = client.register_public_key(1, "test-key".to_string()).await;

        assert!(matches!(result, Err(ApiError::NetworkError(_))));
    }
}
