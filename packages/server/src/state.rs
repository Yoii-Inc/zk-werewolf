use axum::extract::ws::Message;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::models::room::Room;

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<Mutex<HashMap<String, Room>>>,
    pub channel: broadcast::Sender<Message>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<Message>(1000);
        AppState {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            channel: tx,
        }
    }
}

pub struct RoomState {}
