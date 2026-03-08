use crate::models::ProofRequest;
use crate::node::Node;
use crate::proof::ProofManager;
use crate::ProofStatus;
use anyhow::Context;
use axum::extract::{Path, State};
use axum::http::{self, HeaderValue, Method};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::FutureExt;
use mpc_net::MpcMultiNet as Net;
use serde_json::json;
use std::any::Any;
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
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
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind zk-mpc-node HTTP listener on {addr}"))?;
    axum::serve(listener, app)
        .await
        .context("zk-mpc-node HTTP server terminated unexpectedly")?;

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

    println!(
        "Proof request registered: {} (profile: {:?})",
        payload.proof_id,
        payload.circuit_type.circuit_profile()
    );

    let payload_clone = payload.clone();

    // Simulate the network request to generate the proof
    spawn(async move {
        Net::simulate(
            state.node.net.clone(),
            payload_clone.clone(),
            move |_, request| {
                let node_clone = state.node.clone();
                let proof_manager = state.proof_manager.clone();
                async move {
                    println!(
                        "Node {} is generating proof for request: {} (profile: {:?})",
                        node_clone.id,
                        request.proof_id,
                        request.circuit_type.circuit_profile()
                    );

                    let proof_result =
                        AssertUnwindSafe(node_clone.generate_proof(request.clone()))
                            .catch_unwind()
                            .await;

                    match proof_result {
                        Ok(Ok(_)) => {
                            println!(
                                "Proof generation completed successfully for {}",
                                request.proof_id
                            );
                        }
                        Ok(Err(e)) => {
                            eprintln!(
                                "Error during proof generation for {}: {:?}",
                                request.proof_id, e
                            );
                            proof_manager
                                .update_proof_status(
                                    &request.proof_id,
                                    "failed",
                                    Some(format!("Error: {:?}", e)),
                                )
                                .await;
                        }
                        Err(panic_payload) => {
                            let panic_message = panic_payload_to_string(panic_payload);
                            eprintln!(
                                "Panic during proof generation for {}: {}",
                                request.proof_id, panic_message
                            );
                            proof_manager
                                .update_proof_status(
                                    &request.proof_id,
                                    "failed",
                                    Some(format!("Panic: {}", panic_message)),
                                )
                                .await;
                        }
                    }
                }
            },
        )
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
    State(_state): State<AppState>,
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
    (
        http::StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({
            "error": "get_proof_output is not yet implemented",
            "proof_id": proof_id
        })),
    )
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

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return (*msg).to_string();
    }
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.clone();
    }
    "unknown panic payload".to_string()
}
