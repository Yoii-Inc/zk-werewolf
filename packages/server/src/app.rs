use crate::routes;
use crate::state::AppState;
use axum::Router;
use chrono::Duration;
use tokio::time::{interval, Duration as TokioDuration};

pub fn create_app() -> Router {
    create_app_with_state(AppState::new())
}

pub fn create_app_with_state(state: AppState) -> Router {
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = interval(TokioDuration::from_secs(60));
        loop {
            ticker.tick().await;
            let removed =
                crate::services::room_service::cleanup_empty_rooms(&cleanup_state, Duration::minutes(10))
                    .await;
            if removed > 0 {
                tracing::info!("Removed {} empty room(s) that exceeded TTL", removed);
            }
        }
    });

    routes::create_routes(state)
}
