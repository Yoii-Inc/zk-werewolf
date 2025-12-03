use axum::extract::ws::Message;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::models::config::DebugConfig;
use crate::models::{game::Game, room::Room};
use crate::services::node_key::NodeKeyService;
use crate::services::user_service::UserService;

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<Mutex<HashMap<String, Room>>>,
    pub games: Arc<Mutex<HashMap<String, Game>>>,
    pub channel: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>>,
    pub user_service: UserService,
    pub debug_config: Arc<DebugConfig>,
    pub node_key_service: Arc<NodeKeyService>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            games: Arc::new(Mutex::new(HashMap::new())),
            channel: Arc::new(Mutex::new(HashMap::new())),
            user_service: UserService::new(),
            debug_config: Arc::new(DebugConfig::default()),
            node_key_service: Arc::new(NodeKeyService::new()),
        }
    }

    pub async fn get_or_create_room_channel(&self, room_id: &str) -> broadcast::Sender<Message> {
        let mut channels = self.channel.lock().await;
        if let Some(channel) = channels.get(room_id) {
            channel.clone()
        } else {
            let (tx, _) = broadcast::channel(1000);
            channels.insert(room_id.to_string(), tx.clone());
            tx
        }
    }

    pub async fn broadcast_phase_change(
        &self,
        room_id: &str,
        from_phase: &str,
        to_phase: &str,
    ) -> Result<(), String> {
        let tx = self.get_or_create_room_channel(room_id).await;

        let phase_notification = serde_json::json!({
            "message_type": "phase_change",
            "from_phase": from_phase,
            "to_phase": to_phase,
            "room_id": room_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "requires_dummy_request": from_phase == "Night" && to_phase == "DivinationProcessing"
        });

        if let Ok(message_text) = serde_json::to_string(&phase_notification) {
            if let Err(e) = tx.send(Message::Text(message_text)) {
                return Err(format!("Failed to broadcast phase change: {}", e));
            }
        }

        Ok(())
    }

    pub async fn broadcast_computation_result(
        &self,
        room_id: &str,
        computation_type: &str,
        result_data: serde_json::Value,
        target_player_id: Option<String>,
        batch_id: &str,
    ) -> Result<(), String> {
        let tx = self.get_or_create_room_channel(room_id).await;

        let computation_notification = serde_json::json!({
            "message_type": "computation_result",
            "computation_type": computation_type,
            "result_data": result_data,
            "room_id": room_id,
            "target_player_id": target_player_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "batch_id": batch_id
        });

        if let Ok(message_text) = serde_json::to_string(&computation_notification) {
            if let Err(e) = tx.send(Message::Text(message_text)) {
                return Err(format!("Failed to broadcast computation result: {}", e));
            }
        }

        Ok(())
    }
    pub async fn save_chat_message(
        &self,
        room_id: &str,
        message: crate::models::chat::ChatMessage,
    ) -> Result<(), String> {
        let mut games = self.games.lock().await;
        if let Some(game) = games.get_mut(room_id) {
            game.chat_log.add_message(message);
            Ok(())
        } else {
            Err("Game not found".to_string())
        }
    }
}
