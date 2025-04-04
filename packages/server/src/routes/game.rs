use crate::models::role::Role;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::state::AppState;
use crate::{
    models::game::{ChangeRoleRequest, GameResult, NightActionRequest},
    services::game_service,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteAction {
    voter_id: String,
    target_id: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .nest(
            "/:roomid",
            Router::new()
                // ゲームの基本操作
                .route("/start", post(start_game))
                .route("/end", post(end_game_handler))
                // curl http://localhost:8080/api/game/{roomid}/state
                .route("/state", get(get_game_state))
                // ゲームアクション
                .nest(
                    "/actions",
                    Router::new()
                        .route("/vote", post(cast_vote_handler))
                        .route("/night-action", post(night_action_handler)),
                )
                // デバッグ用エンドポイント
                .route("/debug/change-role", post(change_player_role))
                // ゲーム進行の管理
                .route("/phase/next", post(advance_phase_handler))
                .route("/check-winner", get(check_winner_handler)),
        )
        .with_state(state)
}

pub async fn start_game(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::start_game(state, &room_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::NOT_FOUND, Json(message)),
    }
}

pub async fn get_game_state(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // match game_service::get_game_state(state, room_id).await {
    //     Ok(game) => (StatusCode::OK, Json(game)),
    //     Err(message) => (StatusCode::NOT_FOUND, Json(message)),
    // }
    let game = game_service::get_game_state(state, room_id).await.unwrap();
    (StatusCode::OK, Json(game))
    // Err(message) => (StatusCode::NOT_FOUND, Json(message)),
}

async fn end_game_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::end_game(state, room_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::NOT_FOUND, Json(message)),
    }
}

async fn night_action_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(action_req): Json<NightActionRequest>,
) -> impl IntoResponse {
    match game_service::process_night_action(state, &room_id, action_req).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(e)),
    }
}

async fn cast_vote_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(vote_action): Json<VoteAction>,
) -> impl IntoResponse {
    match game_service::handle_vote(
        state,
        &room_id,
        &vote_action.voter_id,
        &vote_action.target_id,
    )
    .await
    {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::BAD_REQUEST, Json(message)),
    }
}

async fn advance_phase_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::advance_game_phase(state, &room_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::BAD_REQUEST, Json(message)),
    }
}

async fn check_winner_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::check_winner(state, &room_id).await {
        Ok(winner) => match winner {
            GameResult::InProgress => (StatusCode::OK, Json("ゲーム進行中".to_string())),
            GameResult::VillagerWin => (StatusCode::OK, Json("村人陣営の勝利".to_string())),
            GameResult::WerewolfWin => (StatusCode::OK, Json("人狼陣営の勝利".to_string())),
        },
        Err(message) => (StatusCode::BAD_REQUEST, Json(message)),
    }
}

pub async fn change_player_role(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<ChangeRoleRequest>,
) -> impl IntoResponse {
    let mut games = state.games.lock().await;

    if let Some(game) = games.get_mut(&room_id) {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == payload.player_id) {
            // 文字列から Role を変換
            let new_role = match payload.new_role.as_str() {
                "村人" => Some(Role::Villager),
                "人狼" => Some(Role::Werewolf),
                "占い師" => Some(Role::Seer),
                _ => None,
            };

            player.role = new_role;
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "役職を変更しました"
                })),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "プレイヤーが見つかりません"
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "ゲームが見つかりません"
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_setup::setup_test_env;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_start_game() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());
        let room_id = crate::services::room_service::create_room(state.clone()).await;

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/start", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_end_game() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());
        let room_id = crate::services::room_service::create_room(state.clone()).await;

        game_service::start_game(state.clone(), &room_id.to_string())
            .await
            .unwrap();

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/end", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
