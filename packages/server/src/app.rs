use crate::routes;
use crate::services::room_service::{self, RoomCleanupPolicy};
use crate::state::AppState;
use axum::Router;
use tokio::time::{interval, Duration as TokioDuration};

fn is_auto_phase_advance_enabled() -> bool {
    // Backward-compatible override:
    // - DEBUG_AUTO_ADVANCE_PHASES=true/false explicitly controls auto progression.
    // - If unset, auto progression is enabled by default even in debug mode.
    std::env::var("DEBUG_AUTO_ADVANCE_PHASES")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

fn default_room_cleanup_policy() -> RoomCleanupPolicy {
    RoomCleanupPolicy::default()
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
    let cleanup_policy = default_room_cleanup_policy();
    tracing::info!(
        "Room cleanup policy: empty_ttl={}m finished_ttl={}m stalled_ttl={}m max_day_count={}",
        cleanup_policy.room_empty_ttl.num_minutes(),
        cleanup_policy.game_finished_ttl.num_minutes(),
        cleanup_policy.game_stalled_ttl.num_minutes(),
        cleanup_policy.game_max_day_count
    );
    tokio::spawn(async move {
        let mut ticker = interval(TokioDuration::from_secs(60));
        loop {
            ticker.tick().await;
            let removed = room_service::cleanup_rooms_and_games(&cleanup_state, cleanup_policy).await;
            if removed.removed_rooms > 0
                || removed.removed_games > 0
                || removed.removed_channels > 0
                || removed.removed_event_stores > 0
                || removed.removed_proof_jobs > 0
            {
                tracing::info!(
                    "Auto cleanup removed rooms={} games={} channels={} event_stores={} proof_jobs={}",
                    removed.removed_rooms,
                    removed.removed_games,
                    removed.removed_channels,
                    removed.removed_event_stores,
                    removed.removed_proof_jobs
                );
            }
        }
    });

    routes::create_routes(state)
}
