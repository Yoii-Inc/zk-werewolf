use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use chrono;
use futures::{sink::SinkExt, stream::StreamExt};
use tracing::info;
use uuid::Uuid;

use crate::models::chat::{ChatMessage, ChatMessageType};
use crate::state::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct WebSocketMessage {
    message_type: String,
    player_id: String,
    player_name: String,
    content: String,
    timestamp: String,
    room_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PhaseChangeNotification {
    message_type: String,
    from_phase: String,
    to_phase: String,
    room_id: String,
    timestamp: String,
    requires_dummy_request: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComputationResultNotification {
    message_type: String,
    computation_type: String, // "divination", "role_assignment", "winning_judge", "anonymous_voting"
    result_data: serde_json::Value,
    room_id: String,
    target_player_id: Option<String>, // 特定のプレイヤーのみに送信する場合
    timestamp: String,
    batch_id: String,
}

impl WebSocketMessage {
    fn to_chat_message(&self) -> ChatMessage {
        let message_type = match self.message_type.as_str() {
            "wolf" => ChatMessageType::Wolf,
            "private" => ChatMessageType::Private,
            "system" => ChatMessageType::System,
            _ => ChatMessageType::Public,
        };

        ChatMessage::new(
            self.player_id.clone(),
            self.player_name.clone(),
            self.content.clone(),
            message_type,
        )
    }
}

pub async fn handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.clone(), room_id))
}

pub async fn handle_socket(ws: WebSocket, state: AppState, room_id: String) {
    info!("New WebSocket connection established for room: {}", room_id);
    let tx = state.get_or_create_room_channel(&room_id).await;

    let (mut sender, mut receiver) = ws.split();
    let mut rx = tx.subscribe();

    let default_player_id = Uuid::new_v4().to_string();
    let room_id_for_send = room_id.clone();
    let room_id_for_receive = room_id.clone();

    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                // JSONメッセージをパースを試みる
                match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(mut ws_message) => {
                        // player_idが空の場合はデフォルトIDを使用
                        if ws_message.player_id.trim().is_empty() {
                            ws_message.player_id = default_player_id.clone();
                        }
                        ws_message.room_id = room_id_for_receive.clone();

                        // チャットメッセージに変換して保存
                        let chat_message = ws_message.to_chat_message();
                        if let Err(e) = state
                            .save_chat_message(&room_id_for_receive, chat_message)
                            .await
                        {
                            eprintln!("Error saving chat message: {}", e);
                        }

                        let response = serde_json::to_string(&ws_message).unwrap();
                        info!(
                            "Received valid message in room {}: {:?}",
                            room_id_for_receive, response
                        );
                        if let Err(e) = tx.send(Message::Text(response)) {
                            eprintln!("Error sending message: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        // 不正なメッセージフォーマットの場合、エラーメッセージを送信
                        let error_message = WebSocketMessage {
                            message_type: "error".to_string(),
                            player_id: "system".to_string(),
                            player_name: "System".to_string(),
                            content: format!("メッセージのフォーマットが不正です: {}", e),
                            timestamp: chrono::Local::now().to_rfc3339(),
                            room_id: room_id_for_receive.clone(),
                        };

                        if let Ok(error_response) = serde_json::to_string(&error_message) {
                            info!("Sending error message: {}", error_response);
                            if let Err(e) = tx.send(Message::Text(error_response)) {
                                eprintln!("Error sending error message: {}", e);
                            }
                        }
                    }
                }
            }
        }
    });

    let room_id_for_send = room_id_for_send.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Message::Text(text) = msg.clone() {
                // メッセージがこのルームのものかを確認
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    if ws_message.room_id != room_id_for_send {
                        continue; // 他のルームのメッセージはスキップ
                    }
                }
            }

            info!("Sending message in room {}: {:?}", room_id_for_send, msg);
            if let Err(e) = sender.send(msg).await {
                eprintln!("Error sending message: {}", e);
                break;
            }
        }
    });

    let _ = tokio::join!(receive_task, send_task);
}
