use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
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

#[derive(Debug, Deserialize)]
pub struct WebSocketConnectQuery {
    #[serde(default)]
    last_event_id: Option<u64>,
    #[serde(default)]
    player_id: Option<String>,
}

fn should_send_event_to_player(
    payload: &serde_json::Value,
    connected_player_id: Option<&str>,
) -> bool {
    let message_type = payload.get("message_type").and_then(|v| v.as_str());
    if message_type != Some("computation_result") {
        return true;
    }

    let target_player_id = payload.get("target_player_id").and_then(|v| v.as_str());
    match target_player_id {
        Some(target) => connected_player_id.is_some_and(|connected| connected == target),
        None => true,
    }
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
    Query(query): Query<WebSocketConnectQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            state.clone(),
            room_id,
            query.last_event_id.unwrap_or(0),
            query.player_id.clone(),
        )
    })
}

pub async fn handle_socket(
    ws: WebSocket,
    state: AppState,
    room_id: String,
    last_event_id: u64,
    connected_player_id: Option<String>,
) {
    info!(
        "New WebSocket connection established for room: {} (player_id={:?})",
        room_id, connected_player_id
    );
    let tx = state.get_or_create_room_channel(&room_id).await;

    let (mut sender, mut receiver) = ws.split();
    let mut rx = tx.subscribe();

    let default_player_id = Uuid::new_v4().to_string();
    let room_id_for_send = room_id.clone();
    let room_id_for_receive = room_id.clone();

    let replay_events = state
        .replay_room_events_since(&room_id, last_event_id)
        .await;
    for event in replay_events {
        if !should_send_event_to_player(&event.payload, connected_player_id.as_deref()) {
            continue;
        }
        match serde_json::to_string(&event) {
            Ok(message_text) => {
                if sender
                    .send(Message::Text(message_text.into()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(e) => eprintln!("Error serializing replay event: {}", e),
        }
    }

    let state_for_receive = state.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
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
                            if let Err(e) = state_for_receive
                                .save_chat_message(&room_id_for_receive, chat_message)
                                .await
                            {
                                eprintln!("Error saving chat message: {}", e);
                            }

                            let payload = match serde_json::to_value(&ws_message) {
                                Ok(payload) => payload,
                                Err(e) => {
                                    eprintln!("Error converting websocket message to json: {}", e);
                                    continue;
                                }
                            };
                            info!(
                                "Received valid message in room {}: {:?}",
                                room_id_for_receive, ws_message
                            );
                            if let Err(e) = state_for_receive
                                .publish_room_event(&room_id_for_receive, payload)
                                .await
                            {
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

                            info!("Sending error message: {:?}", error_message);
                            let error_payload = match serde_json::to_value(&error_message) {
                                Ok(payload) => payload,
                                Err(err) => {
                                    eprintln!("Error converting error message to json: {}", err);
                                    continue;
                                }
                            };
                            if let Err(err) = state_for_receive
                                .publish_room_event(&room_id_for_receive, error_payload)
                                .await
                            {
                                eprintln!("Error sending error message: {}", err);
                            }
                        }
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket close received for room {}", room_id_for_receive);
                    break;
                }
                _ => {}
            }
        }
    });

    let room_id_for_send = room_id_for_send.clone();
    let connected_player_id_for_send = connected_player_id.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Message::Text(ref text) = msg {
                match serde_json::from_str::<crate::state::RoomEventEnvelope>(text) {
                    Ok(event) => {
                        if !should_send_event_to_player(
                            &event.payload,
                            connected_player_id_for_send.as_deref(),
                        ) {
                            continue;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error parsing room event envelope: {}", e);
                    }
                }
            }
            info!("Sending message in room {}: {:?}", room_id_for_send, msg);
            if let Err(e) = sender.send(msg).await {
                let err_text = e.to_string();
                if err_text.contains("closed connection") {
                    info!("WebSocket already closed for room {}. closing sender task.", room_id_for_send);
                } else {
                    eprintln!("Error sending message: {}", err_text);
                }
                break;
            }
        }
    });

    let _ = tokio::join!(receive_task, send_task);
}
