use std::collections::HashMap;

use crate::state::AppState;
use crate::utils::websocket;
use crate::{models::room::Room, services::room_service};
use axum::extract::{ws, Path};
use axum::routing::delete;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};


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
        .route("/:roomid", get(get_room_info))
        
        // ルーム参加
        // curl -X POST http://localhost:8080/api/room/{roomid}/join
        .route("/:roomid/join", post(join_room))
        
        // ルーム削除
        // curl -X DELETE http://localhost:8080/api/room/{roomid}/delete
        .route("/:roomid/delete", delete(delete_room))
        
        // WebSocket接続
        // websocat ws://localhost:8080/api/room/ws
        .route("/ws", get(websocket::handler))
        .with_state(state)
}

async fn create_room(State(state): State<AppState>) -> Json<String> {
    let room_id = room_service::create_room(state).await;
    Json(format!("Room created with ID: {}", room_id))
}

async fn join_room(State(state): State<AppState>, Path(room_id): Path<String>) -> Json<String> {
    let success = room_service::join_room(state, &room_id).await;
    if success {
        Json("Successfully joined room".to_string())
    } else {
        Json("Failed to join room".to_string())
    }
}

async fn delete_room(State(state): State<AppState>, Path(room_id): Path<String>) -> Json<String> {
    let success = room_service::delete_room(state, &room_id).await;
    if success {
        Json(format!("Room {} deleted successfully", room_id))
    } else {
        Json(format!("Failed to delete room {}", room_id))
    }
}

async fn get_rooms(State(state): State<AppState>) -> Json<HashMap<String, Room>> {
    let rooms = room_service::get_rooms(&state).await;
    Json(rooms)
}

async fn get_room_info(State(state): State<AppState>, Path(room_id): Path<String>) -> Json<Room> {
    let room = room_service::get_room_info(&state, &room_id).await;
    Json(room)
}

#[cfg(test)]
mod tests {
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
    async fn test_create_room() {
        let state = AppState::new();
        let app = routes(state);

        let request = Request::builder()
            .method("POST")
            .uri("/create")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), 100).await.unwrap();
        let room_id = String::from_utf8(body.to_vec()).unwrap();
        assert!(room_id.contains("Room created with ID:"));
    }

    #[tokio::test]
    async fn test_get_rooms() {
        let state = AppState::new();
        let app = routes(state.clone());

        room_service::create_room(state).await;

        let request = Request::builder()
            .method("GET")
            .uri("/rooms")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), 100).await.unwrap();
        let rooms: HashMap<String, Room> = serde_json::from_slice(&body).unwrap();
        assert!(!rooms.is_empty());
    }

    // #[tokio::test]
    // async fn test_websocket_connection() {
    //     use tokio_tungstenite::connect_async;
        
    //     let state = AppState::new();
    //     let app = routes(state);
        
    //     let server = tokio::spawn(async move {
    //         let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    //         let (stream, _) = listener.accept().await.unwrap();
    //         let io = TokioIo::new(stream);
            
    //         let service = app.into_make_service();
    //         let _ = http1::Builder::new()
    //             .serve_connection(io, service)
    //             .await;
    //     });

    //     let (ws_stream, _) = connect_async("ws://localhost:8080/ws")
    //         .await
    //         .expect("Failed to connect");
            
    //     assert!(ws_stream.can_read());

    //     server.abort();
    // }
}
