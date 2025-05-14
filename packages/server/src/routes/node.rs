use crate::{
    models::node::{ErrorResponse, NodeKey, RegisterKeyResponse},
    state::AppState,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

pub async fn register_key(
    State(state): State<AppState>,
    Json(payload): Json<NodeKey>,
) -> impl axum::response::IntoResponse {
    match state
        .node_key_service
        .store_key(payload.node_id, payload.public_key.clone())
    {
        Ok(_) => (
            StatusCode::OK,
            Json(RegisterKeyResponse {
                success: true,
                node_id: payload.node_id,
                public_key: payload.public_key,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: e,
            }),
        )
            .into_response(),
    }
}

pub async fn get_node_key(
    State(state): State<AppState>,
    path: axum::extract::Path<u32>,
) -> impl IntoResponse {
    let node_id = path.0;
    match state.node_key_service.get_key(node_id) {
        Some(public_key) => (
            StatusCode::OK,
            Json(NodeKey {
                node_id,
                public_key,
            }),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: "Node key not found".to_string(),
            }),
        )
            .into_response(),
    }
}

pub async fn get_all_keys(State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(state.node_key_service.get_all_keys())).into_response()
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(register_key))
        .route("/:node_id", get(get_node_key))
        .route("/", get(get_all_keys))
        .with_state(state)
}
