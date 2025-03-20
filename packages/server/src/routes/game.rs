use axum::{
    extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router
};
use serde::{Deserialize, Serialize};

use crate::{models::game::{Game, GameAction}, services::game_service};
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameStartRequest {
    room_id: String,
    player_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameActionRequest {
    room_id: String,
    action: GameAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    room_id: String,
    player_id: String,
    vote: bool,
}


pub fn routes(state: AppState) -> Router {
    Router::new()
        // roomidで指定されたゲームを開始
        // curl -X POST http://localhost:8080/api/game/{roomid}/start
        .route("/:roomid/start", post(start_game))
        // roomidで指定されたゲームの状態を取得
        // curl http://localhost:8080/api/game/{roomid}/state
        .route("/:roomid/state", get(get_game_state))
        // roomidで指定されたゲームに対してアクション(投票やゲーム終了)を実行
        // curl -X POST -H "Content-Type: application/json" -d '{"action": "vote"}' http://localhost:8080/api/game/{roomid}/action
        .route("/:roomid/action", post(post_game_action_handler))
        // roomidで指定されたゲームに対して投票を実行
        // curl -X POST -H "Content-Type: application/json" -d '{"player_id": "123", "vote": true}' http://localhost:8080/api/game/{roomid}/vote
        .route("/:roomid/vote", post(post_vote_handler))
        // roomidで指定されたゲームを終了
        // curl -X POST http://localhost:8080/api/game/{roomid}/end
        .route("/:roomid/end", post(end_game_handler))
        .with_state(state)
}

pub async fn start_game(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::start_game(state, &room_id).await {
        Ok(message) => (StatusCode::OK, message),
        Err(message) => (StatusCode::NOT_FOUND, message),
    }
}

pub async fn get_game_state(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match game_service::get_game_state(state, room_id).await {
        Ok(state) => (StatusCode::OK, Json(state)),
        Err(message) => todo!(), //(StatusCode::NOT_FOUND, Json(message)),
    }
}

async fn post_game_action_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(action): Json<GameActionRequest>,
) -> impl IntoResponse {
    match game_service::post_game_action(state, room_id, action.action).await {
        Ok(message) => (StatusCode::OK, message),
        Err(message) => (StatusCode::NOT_FOUND, message),
    }
}

async fn post_vote_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(vote): Json<VoteRequest>,
) -> impl IntoResponse {
    match game_service::post_vote(state, room_id, vote.player_id, vote.vote).await {
        Ok(message) => (StatusCode::OK, message),
        Err(message) => (StatusCode::NOT_FOUND, message),
    }
}

async fn end_game_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::end_game(state, room_id).await {
        Ok(message) => (StatusCode::OK, message),
        Err(message) => (StatusCode::NOT_FOUND, message),
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{models::room::Room, services::room_service};

    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    // use hyper::Server;
    use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_start_game() {
        let state = AppState::new();
        let app = routes(state.clone());

        let room_id = room_service::create_room(state.clone()).await;

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/start", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), 100).await.unwrap();
        let message = String::from_utf8(body.to_vec()).unwrap();
        assert!(message.contains("Game started successfully"));
    }

    #[tokio::test]
    async fn test_end_game() {
        let state = AppState::new();
        let app = routes(state.clone());

        let room_id = room_service::create_room(state.clone()).await;
        game_service::start_game(state.clone(), &room_id.to_string()).await.unwrap();

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/end", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), 100).await.unwrap();
        let message = String::from_utf8(body.to_vec()).unwrap();
        assert!(message.contains("Game ended successfully"));
    }
}
