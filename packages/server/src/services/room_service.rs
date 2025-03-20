use std::collections::HashMap;

use crate::{
    models::{
        room::{Room, RoomStatus},
        player::Player,
    },
    state::AppState,
};

pub async fn create_room(state: AppState) -> u32 {
    let mut rooms = state.rooms.lock().await;
    let new_id = rooms.keys().map(|k| k.parse::<u32>().unwrap()).max().unwrap_or(0) + 1;
    let new_room = Room::new(new_id.to_string(), None, None);
    rooms.insert(new_id.to_string(), new_room);
    new_id
}

pub async fn join_room(state: AppState, room_id: &str, player_id: u32) -> bool {
    let mut rooms = state.rooms.lock().await;
    
    if let Some(room) = rooms.get_mut(room_id) {
        // ルームの状態がOpenか確認
        if room.status != RoomStatus::Open {
            return false;
        }

        // プレイヤー数の上限チェック
        if room.players.len() >= room.max_players {
            return false;
        }

        // 既に参加しているプレイヤーかチェック
        if room.players.iter().any(|p| p.id == player_id) {
            return false;
        }

        // 新しいプレイヤーを追加
        let player = Player {
            id: player_id,
            name: format!("Player {}", player_id),
            role: String::new(),
        };
        room.players.push(player);
        true
    } else {
        false
    }
}

pub async fn leave_room(state: AppState, room_id: &str, player_id: u32) -> bool {
    let mut rooms = state.rooms.lock().await;
    
    if let Some(room) = rooms.get_mut(room_id) {
        // プレイヤーが存在するかチェック
        let player_index = room.players.iter().position(|p| p.id == player_id);
        
        if let Some(index) = player_index {
            // プレイヤーを削除
            room.players.remove(index);
            true
        } else {
            false
        }
    } else {
        false
    }
}

pub async fn get_rooms(state: &AppState) -> HashMap<String, Room> {
    state.rooms.lock().await.clone()
}

pub async fn get_room_info(state: &AppState, room_id: &str) -> Room {
    let rooms = state.rooms.lock().await;
    rooms.get(room_id).unwrap().clone()
}

pub async fn delete_room(state: AppState, room_id: &str) -> bool {
    let mut rooms = state.rooms.lock().await;
    rooms.remove(room_id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // #[tokio::test]
    // async fn test_create_room() {
    //     let state = AppState {
    //         rooms: Arc::new(Mutex::new(vec![])),
    //     };
    //     let room_id = create_room(state.clone()).await;
    //     assert_eq!(room_id, 1);
    //     assert_eq!(state.rooms.lock().await.len(), 1);
    // }
}
