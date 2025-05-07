use axum::extract::ws::Message;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::models::config::DebugConfig;
use crate::models::{game::Game, room::Room};
use crate::services::user_service::UserService;

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<Mutex<HashMap<String, Room>>>,
    pub games: Arc<Mutex<HashMap<String, Game>>>,
    pub channel: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>>,
    pub user_service: UserService,
    pub debug_config: Arc<DebugConfig>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            games: Arc::new(Mutex::new(HashMap::new())),
            channel: Arc::new(Mutex::new(HashMap::new())),
            user_service: UserService::new(),
            debug_config: Arc::new(DebugConfig::default()),
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
