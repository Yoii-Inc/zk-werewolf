use crate::models::ProofRequest;
use crate::node::Node;
use crate::proof::ProofManager;
use mpc_net::MpcMultiNet as Net;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod api_client;

pub use api_client::*;

pub async fn handle_client(
    mut socket: TcpStream,
    proof_manager: Arc<ProofManager>,
    node: Arc<Node<TcpStream>>,
) {
    let mut buffer = vec![0; 1024];
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    let _size = socket.read(&mut buffer).await.unwrap();
    let result = req
        .parse(&buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        .unwrap();

    match (req.method, req.path) {
        (Some("POST"), Some("/")) => {
            let body_start = match result {
                httparse::Status::Complete(size) => size,
                httparse::Status::Partial => {
                    return;
                }
            };
            let body = String::from_utf8_lossy(&buffer[body_start..])
                .trim_matches(char::from(0))
                .to_string();

            if let Ok(request) = serde_json::from_str::<ProofRequest>(&body) {
                proof_manager.register_proof_request(request.clone()).await;

                let response = json!({
                    "success": true,
                    "message": "Request accepted successfully",
                });

                let response_str = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\r\n{}",
                    response.to_string().len(),
                    response
                );
                socket.write_all(response_str.as_bytes()).await.unwrap();

                Net::simulate(node.net.clone(), request, move |_, request| {
                    let node_clone = node.clone();
                    async move {
                        node_clone.generate_proof(request).await;
                    }
                })
                .await;
            }
        }

        (Some("GET"), Some(path)) if path.starts_with("/proof/") => {
            let path = path.trim_start_matches("/proof/");

            // /proof/{id}/output エンドポイントの処理
            if path.ends_with("/output") {
                let proof_id = path.trim_end_matches("/output");
                if let Some(status) = proof_manager.get_proof_status(proof_id).await {
                    if let Some(output) = &status.output {
                        let response_str = format!(
                            "HTTP/1.1 200 OK\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\r\n{}",
                            serde_json::to_string(&output).unwrap().len(),
                            serde_json::to_string(&output).unwrap()
                        );
                        socket.write_all(response_str.as_bytes()).await.unwrap();
                        return;
                    }
                }
                let response = json!({
                    "error": "Proof output not found"
                });
                let response_str = format!(
                    "HTTP/1.1 404 Not Found\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\r\n{}",
                    response.to_string().len(),
                    response
                );
                socket.write_all(response_str.as_bytes()).await.unwrap();
                return;
            }

            // 通常の /proof/{id} エンドポイントの処理
            let proof_id = path;
            if let Some(status) = proof_manager.get_proof_status(proof_id).await {
                let response_str = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\r\n{}",
                    serde_json::to_string(&status).unwrap().len(),
                    serde_json::to_string(&status).unwrap()
                );
                socket.write_all(response_str.as_bytes()).await.unwrap();
            } else {
                let response = json!({
                    "error": "Proof not found"
                });
                let response_str = format!(
                    "HTTP/1.1 404 Not Found\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\r\n{}",
                    response.to_string().len(),
                    response
                );
                socket.write_all(response_str.as_bytes()).await.unwrap();
            }
        }

        _ => {
            let response = json!({
                "error": "Invalid request"
            });
            let response_str = format!(
                "HTTP/1.1 400 Bad Request\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\r\n{}",
                response.to_string().len(),
                response
            );
            socket.write_all(response_str.as_bytes()).await.unwrap();
        }
    }
}
