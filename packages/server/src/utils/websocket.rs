use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

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

pub async fn handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.channel.clone()))
}

pub async fn handle_socket(ws: WebSocket, tx: broadcast::Sender<Message>) {
    let (mut sender, mut receiver) = ws.split(); // WebSocketの送信側と受信側に分割

    // let (tx, _) = broadcast::channel(1024); // メッセージブロードキャストチャネル

    let mut rx = tx.subscribe(); // メッセージブロードキャストチャネルの受信側

    let player_id = Uuid::new_v4().to_string();

    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                // JSON メッセージをパース
                if let Ok(mut ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    ws_message.player_id = player_id.clone();
                    let response = serde_json::to_string(&ws_message).unwrap();
                    info!("Received message: {:?}", response);
                    if let Err(e) = tx.send(Message::Text(response)) {
                        eprintln!("Error sending message: {}", e);
                        break;
                    }
                }
                // メッセージをブロードキャスト
                // let response = format!("Player {}: {}", player_id, text);
                // let response = format!("{}", text);
                // info!("Received message: {:?}", response);
                // if let Err(e) = tx.send(Message::Text(response)) {
                //     eprintln!("Error sending message: {}", e);
                //     break;
                // }
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            info!("Sending message: {:?}", msg);
            if let Err(e) = sender.send(msg).await {
                eprintln!("Error sending message: {}", e);
                break;
            }
        }
    });

    let _ = tokio::join!(receive_task, send_task);
}
