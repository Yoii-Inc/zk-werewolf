use crate::state::AppState;
use axum::Router;

mod game;
mod room;

pub fn create_routes(state: AppState) -> Router {
    Router::new().nest("/api/room", room::routes(state.clone()))
    .nest("/api/game", game::routes(state.clone()))
}
