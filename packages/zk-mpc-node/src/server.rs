use crate::models::ProofRequest;
use crate::node::Node;
use crate::proof::ProofManager;
use crate::ProofStatus;
use axum::extract::{Path, State};
use axum::http::{self, HeaderValue, Method};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use mpc_net::MpcMultiNet as Net;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::spawn;
use tower_http::cors::CorsLayer;

pub mod api_client;

pub use api_client::*;

#[derive(Clone)]
pub struct AppState {
    pub proof_manager: Arc<ProofManager>,
    pub node: Arc<Node<TcpStream>>,
}

pub async fn run_server(addr: &SocketAddr, state: AppState) -> Result<(), anyhow::Error> {
    let origins = ["http://localhost:3000".parse::<HeaderValue>().unwrap()];
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION]);

    // build our application with a single route
    let app = Router::new()
        .route("/", post(handle_proof_request))
        .route("/proof/:proof_id", get(get_proof_status))
        .route("/proof/:proof_id/output", get(get_proof_output))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn handle_proof_request(
    State(state): State<AppState>,
    Json(payload): Json<ProofRequest>,
) -> impl IntoResponse {
    // Handle the proof request
    state
        .proof_manager
        .register_proof_request(payload.clone())
        .await;

    println!("Proof request registered: {}", payload.proof_id);

    let payload_clone = payload.clone();

    // Simulate the network request to generate the proof
    spawn(async move {
        Net::simulate(state.node.net.clone(), payload_clone, move |_, request| {
            let node_clone = state.node.clone();
            async move {
                println!(
                    "Node {} is generating proof for request: {}",
                    node_clone.id, request.proof_id
                );
                node_clone.generate_proof(request).await;
            }
        })
        .await
    });

    (
        http::StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Proof request accepted",
            "proof_id": payload.proof_id
        })),
    )
}

async fn get_proof_output(
    State(state): State<AppState>,
    Path(proof_id): Path<String>,
) -> impl IntoResponse {
    // if let Some(output) = state.proof_manager.get_proof_output(&proof_id).await {
    //     (
    //         http::StatusCode::OK,
    //         axum::Json(format!("{}", serde_json::to_string(&output).unwrap())),
    //     )
    // } else {
    //     (
    //         http::StatusCode::NOT_FOUND,
    //         // axum::Json(json!({"error": "Proof output not found"})),
    //         axum::Json(format!(
    //             "{{\"error\": \"Proof output for {} not found\"}}",
    //             proof_id
    //         )),
    //     )
    // }
    todo!()
}

async fn get_proof_status(
    State(state): State<AppState>,
    Path(proof_id): Path<String>,
) -> impl IntoResponse {
    if let Some(status) = state.proof_manager.get_proof_status(&proof_id).await {
        (http::StatusCode::OK, axum::Json(status))
    } else {
        let proof_status = ProofStatus {
            state: "not_found".to_string(),
            proof_id: proof_id.clone(),
            message: Some(format!("Proof {} not found", proof_id)),
            output: None,
        };
        (http::StatusCode::NOT_FOUND, axum::Json(proof_status))
    }
}
