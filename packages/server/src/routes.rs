use crate::state::AppState;
use axum::Router;

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
    // "/health" is registered in main.rs, outside the TraceLayer/CORS
    // stack, so ALB's 30s health-check probes don't flood the access log.
}
