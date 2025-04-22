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
    pub channel: broadcast::Sender<Message>,
    pub user_service: UserService,
    pub debug_config: Arc<DebugConfig>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<Message>(1000);
        AppState {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            games: Arc::new(Mutex::new(HashMap::new())),
            channel: tx,
            user_service: UserService::new(),
            debug_config: Arc::new(DebugConfig::default()),
        }
    }
}
