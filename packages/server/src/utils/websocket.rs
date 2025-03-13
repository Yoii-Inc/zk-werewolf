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

use crate::state::AppState;

pub async fn handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.channel.clone()))
}

pub async fn handle_socket(ws: WebSocket, tx: broadcast::Sender<Message>) {
    let (mut sender, mut receiver) = ws.split(); // WebSocketの送信側と受信側に分割

    // let (tx, _) = broadcast::channel(1024); // メッセージブロードキャストチャネル

    let mut rx = tx.subscribe(); // メッセージブロードキャストチャネルの受信側

    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // メッセージをブロードキャスト
            info!("Received message: {:?}", msg);
            if let Err(e) = tx.send(msg) {
                eprintln!("Error sending message: {}", e);
                break;
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
