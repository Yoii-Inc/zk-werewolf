use axum::extract::ws::Message;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::blockchain::BlockchainClient;
use crate::models::config::DebugConfig;
use crate::models::{game::Game, room::Room};
use crate::services::node_key::NodeKeyService;
use crate::services::proof_job_service::{ProofJobService, ProofJobStatus};
use crate::services::user_service::UserService;
use crate::utils::config::CONFIG;

const ROOM_EVENT_HISTORY_LIMIT: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEventEnvelope {
    pub event_id: u64,
    pub room_id: String,
    pub timestamp: String,
    pub payload: Value,
}

#[derive(Default)]
struct RoomEventStore {
    next_event_id: u64,
    events: Vec<RoomEventEnvelope>,
}

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<Mutex<HashMap<String, Room>>>,
    pub games: Arc<Mutex<HashMap<String, Game>>>,
    pub channel: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>>,
    room_event_store: Arc<Mutex<HashMap<String, RoomEventStore>>>,
    pub user_service: UserService,
    pub debug_config: Arc<DebugConfig>,
    pub node_key_service: Arc<NodeKeyService>,
    pub proof_job_service: Arc<ProofJobService>,
    pub blockchain_client: Arc<BlockchainClient>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            games: Arc::new(Mutex::new(HashMap::new())),
            channel: Arc::new(Mutex::new(HashMap::new())),
            room_event_store: Arc::new(Mutex::new(HashMap::new())),
            user_service: UserService::new(),
            debug_config: Arc::new(DebugConfig::default()),
            node_key_service: Arc::new(NodeKeyService::new()),
            proof_job_service: Arc::new(ProofJobService::new()),
            blockchain_client: Arc::new(BlockchainClient::new(&CONFIG)),
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

    async fn create_room_event(&self, room_id: &str, payload: Value) -> RoomEventEnvelope {
        let mut stores = self.room_event_store.lock().await;
        let store = stores.entry(room_id.to_string()).or_default();
        store.next_event_id += 1;

        let event = RoomEventEnvelope {
            event_id: store.next_event_id,
            room_id: room_id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            payload,
        };

        store.events.push(event.clone());
        if store.events.len() > ROOM_EVENT_HISTORY_LIMIT {
            let drop_count = store.events.len() - ROOM_EVENT_HISTORY_LIMIT;
            store.events.drain(0..drop_count);
        }

        event
    }

    pub async fn publish_room_event(&self, room_id: &str, payload: Value) -> Result<u64, String> {
        let event = self.create_room_event(room_id, payload).await;
        let message_text = serde_json::to_string(&event)
            .map_err(|e| format!("Failed to serialize room event: {}", e))?;

        let tx = self.get_or_create_room_channel(room_id).await;
        tx.send(Message::Text(message_text.into()))
            .map_err(|e| format!("Failed to broadcast room event: {}", e))?;

        Ok(event.event_id)
    }

    pub async fn replay_room_events_since(
        &self,
        room_id: &str,
        last_event_id: u64,
    ) -> Vec<RoomEventEnvelope> {
        let stores = self.room_event_store.lock().await;
        let Some(store) = stores.get(room_id) else {
            return Vec::new();
        };

        store
            .events
            .iter()
            .filter(|event| event.event_id > last_event_id)
            .cloned()
            .collect()
    }

    pub async fn broadcast_phase_change(
        &self,
        room_id: &str,
        from_phase: &str,
        to_phase: &str,
    ) -> Result<(), String> {
        let phase_notification = serde_json::json!({
            "message_type": "phase_change",
            "from_phase": from_phase,
            "to_phase": to_phase,
            "room_id": room_id,
            "timestamp": Utc::now().to_rfc3339(),
            "requires_dummy_request": from_phase == "Night" && to_phase == "DivinationProcessing"
        });

        self.publish_room_event(room_id, phase_notification)
            .await
            .map(|_| ())
    }

    pub async fn broadcast_commitments_ready(
        &self,
        room_id: &str,
        commitments_count: usize,
        total_players: usize,
    ) -> Result<(), String> {
        let commitments_ready_notification = serde_json::json!({
            "message_type": "commitments_ready",
            "room_id": room_id,
            "commitments_count": commitments_count,
            "total_players": total_players,
            "timestamp": Utc::now().to_rfc3339(),
        });

        self.publish_room_event(room_id, commitments_ready_notification)
            .await
            .map(|_| ())
    }

    pub async fn broadcast_computation_result(
        &self,
        room_id: &str,
        computation_type: &str,
        result_data: serde_json::Value,
        target_player_id: Option<String>,
        batch_id: &str,
    ) -> Result<(), String> {
        let computation_notification = serde_json::json!({
            "message_type": "computation_result",
            "computation_type": computation_type,
            "result_data": result_data,
            "room_id": room_id,
            "target_player_id": target_player_id.clone(),
            "timestamp": Utc::now().to_rfc3339(),
            "batch_id": batch_id
        });

        self.publish_room_event(room_id, computation_notification)
            .await
            .map(|_| ())
    }

    pub async fn broadcast_game_reset(&self, room_id: &str) -> Result<(), String> {
        let reset_notification = serde_json::json!({
            "message_type": "game_reset",
            "room_id": room_id,
            "timestamp": Utc::now().to_rfc3339(),
        });

        self.publish_room_event(room_id, reset_notification)
            .await
            .map(|_| ())
    }

    pub async fn broadcast_room_state_changed(
        &self,
        room_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "message_type": "room_state_changed",
            "room_id": room_id,
            "reason": reason,
            "timestamp": Utc::now().to_rfc3339(),
        });
        self.publish_room_event(room_id, payload).await.map(|_| ())
    }

    pub async fn broadcast_proof_job_status(
        &self,
        room_id: &str,
        status: &ProofJobStatus,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "message_type": "proof_job_status",
            "room_id": room_id,
            "batch_id": status.batch_id,
            "state": status.state,
            "attempt_count": status.attempt_count,
            "last_error": status.last_error,
            "job_node_status": status.job_node_status,
            "updated_at": status.updated_at.to_rfc3339(),
        });
        self.publish_room_event(room_id, payload).await.map(|_| ())
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
