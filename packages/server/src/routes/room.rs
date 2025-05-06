use crate::{services::room_service, state::AppState, utils::websocket};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CreateRoomRequest {
    pub name: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        // ルーム作成
        // curl -X POST http://localhost:8080/api/room/create
        .route("/create", post(create_room))
        // ルーム一覧取得
        // curl http://localhost:8080/api/room/rooms
        .route("/rooms", get(get_rooms))
        // 特定のルーム情報取得
        // curl http://localhost:8080/api/room/{roomid}
        .route("/:id", get(get_room_info))
        // ルーム参加
        // curl -X POST http://localhost:8080/api/room/{roomid}/join/{playerid}
        .route("/:id/join/:playerid", post(join_room))
        // ルーム脱退
        // curl -X POST http://localhost:8080/api/room/{roomid}/leave/{playerid}
        .route("/:id/leave/:playerid", post(leave_room))
        // ルーム削除
        // curl -X DELETE http://localhost:8080/api/room/{roomid}/delete
        .route("/:id/delete", delete(delete_room))
        // WebSocket接続
        // websocat ws://localhost:8080/api/room/ws
        .route("/ws", get(websocket::handler))
        // ルームの準備完了トグル
        // curl -X POST http://localhost:8080/api/room/{roomid}/ready/{playerid}
        .route("/:id/ready/:playerid", post(toggle_ready))
        .with_state(state)
}

pub async fn create_room(
    State(state): State<AppState>,
    Json(payload): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    let room_id = room_service::create_room(state, Some(payload.name)).await;
    (
        StatusCode::OK,
        Json(format!("Room created with ID: {}", room_id)),
    )
}

async fn get_rooms(State(state): State<AppState>) -> impl IntoResponse {
    let rooms = room_service::get_rooms(&state).await;
    (StatusCode::OK, Json(rooms))
}

async fn get_room_info(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let room = room_service::get_room_info(&state, &room_id).await;
    (StatusCode::OK, Json(room))
}

pub async fn join_room(
    State(state): State<AppState>,
    Path((room_id, player_id)): Path<(String, String)>,
) -> impl IntoResponse {
    // ユーザー認証情報からユーザー名を取得（auth_middlewareで設定される）
    let user = state
        .user_service
        .get_user_by_id(&player_id)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json("User not found")))
        .unwrap();

    let success = room_service::join_room(state, &room_id, &player_id, &user.username).await;
    if success {
        (StatusCode::OK, Json("Successfully joined room"))
    } else {
        (StatusCode::BAD_REQUEST, Json("Failed to join room"))
    }
}

pub async fn leave_room(
    State(state): State<AppState>,
    Path((room_id, player_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let success = room_service::leave_room(state, &room_id, &player_id).await;
    if success {
        (StatusCode::OK, Json("Successfully left room"))
    } else {
        (StatusCode::BAD_REQUEST, Json("Failed to leave room"))
    }
}

async fn delete_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let success = room_service::delete_room(state, &room_id).await;
    if success {
        (
            StatusCode::OK,
            Json(format!("Room {} deleted successfully", room_id)),
        )
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(format!("Failed to delete room {}", room_id)),
        )
    }
}

pub async fn toggle_ready(
    State(state): State<AppState>,
    Path((room_id, player_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match room_service::toggle_ready(state, &room_id, &player_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::room::Room, utils::test_setup::setup_test_env};
    use axum::{body::to_bytes, body::Body, http::Request};
    use std::collections::HashMap;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_room() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state);

        let create_request = CreateRoomRequest {
            name: "テストルーム".to_string(),
        };

        let request = Request::builder()
            .method("POST")
            .uri("/create")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let room_id = String::from_utf8(body.to_vec()).unwrap();
        assert!(room_id.contains("Room created with ID:"));
    }

    #[tokio::test]
    async fn test_get_rooms() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());

        // テスト用のルームを作成
        let room_id = room_service::create_room(state, None).await;

        let request = Request::builder()
            .method("GET")
            .uri("/rooms")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let rooms: HashMap<String, Room> =
            serde_json::from_slice(&body).expect("Failed to parse response body");

        assert!(!rooms.is_empty());
        assert!(rooms.contains_key(&room_id.to_string()));
    }
}
