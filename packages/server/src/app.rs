use crate::routes;
use axum::Router;

pub fn create_app() -> Router {
    let state = crate::state::AppState::new();
    routes::create_routes(state)
}
