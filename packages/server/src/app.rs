use crate::routes;
use crate::state::AppState;
use axum::Router;
use chrono::Duration;
use tokio::time::{interval, Duration as TokioDuration};

fn is_auto_phase_advance_enabled() -> bool {
    // Backward-compatible override:
    // - DEBUG_AUTO_ADVANCE_PHASES=true/false explicitly controls auto progression.
    // - If unset, auto progression is enabled by default even in debug mode.
    std::env::var("DEBUG_AUTO_ADVANCE_PHASES")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

pub fn create_app() -> Router {
    create_app_with_state(AppState::new())
}

pub fn create_app_with_state(state: AppState) -> Router {
    let auto_phase_state = state.clone();
    if is_auto_phase_advance_enabled() {
        tokio::spawn(async move {
            let mut ticker = interval(TokioDuration::from_secs(1));
            loop {
                ticker.tick().await;
                crate::services::game_service::auto_advance_due_phases(auto_phase_state.clone())
                    .await;
            }
        });
    } else {
        tracing::info!("Auto phase advance is disabled by DEBUG_AUTO_ADVANCE_PHASES");
    }

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = interval(TokioDuration::from_secs(60));
        loop {
            ticker.tick().await;
            let removed = crate::services::room_service::cleanup_empty_rooms(
                &cleanup_state,
                Duration::minutes(10),
            )
            .await;
            if removed > 0 {
                tracing::info!("Removed {} empty room(s) that exceeded TTL", removed);
            }
        }
    });

    routes::create_routes(state)
}
