use crate::state::AppState;
use axum::{routing::get, Router};

pub mod game;
pub mod health;
pub mod node;
pub mod room;
pub mod user;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .nest("/api/room", room::routes(state.clone()))
        .nest("/api/game", game::routes(state.clone()))
        .nest("/api/users", user::routes(state.clone()))
        .nest("/api/nodes/keys", node::routes(state.clone()))
        .route("/health", get(health::health_check))
}
