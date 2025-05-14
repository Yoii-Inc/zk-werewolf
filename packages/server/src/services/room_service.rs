use crate::{
    models::{
        player::Player,
        role::Role,
        room::{Room, RoomStatus},
    },
    state::AppState,
};
use std::collections::HashMap;

pub async fn create_room(state: AppState, name: Option<String>) -> u32 {
    let mut rooms = state.rooms.lock().await;
    let new_id = rooms
        .keys()
        .map(|k| k.parse::<u32>().unwrap())
        .max()
        .unwrap_or(0)
        + 1;
    let new_room = Room::new(new_id.to_string(), name, None);
    rooms.insert(new_id.to_string(), new_room);
    new_id
}

pub async fn join_room(state: AppState, room_id: &str, player_id: &str, player_name: &str) -> bool {
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

        // 既に参加しているプレイヤーの場合
        if let Some(existing_player) = room.players.iter_mut().find(|p| p.id == player_id) {
            // 名前を更新して再接続として扱う
            existing_player.name = player_name.to_string();
            return true; // 再接続成功
        }

        // 新しいプレイヤーを追加
        let player = Player {
            id: player_id.to_string(),
            name: player_name.to_string(),
            role: Some(Role::Villager),
            is_dead: false,
            is_ready: false,
        };
        room.players.push(player);
        true
    } else {
        false
    }
}

pub async fn leave_room(state: AppState, room_id: &str, player_id: &str) -> bool {
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

pub async fn toggle_ready(
    state: AppState,
    room_id: &str,
    player_id: &str,
) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        // プレイヤーの存在確認
        if let Some(player) = room.players.iter_mut().find(|p| p.id == player_id) {
            // ready状態を切り替え
            player.is_ready = !player.is_ready;

            // 全員の準備が完了しているかチェック
            let all_ready = room.players.len() >= 4 && room.players.iter().all(|p| p.is_ready);

            if all_ready {
                room.status = RoomStatus::Ready;
                Ok("全員の準備が完了しました。".to_string())
            } else {
                Ok("準備状態を切り替えました。".to_string())
            }
        } else {
            Err("プレイヤーが見つかりません。".to_string())
        }
    } else {
        Err("ルームが見つかりません。".to_string())
    }
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
