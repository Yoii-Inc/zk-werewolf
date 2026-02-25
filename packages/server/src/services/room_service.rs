use crate::{
    models::{
        player::Player,
        room::{Room, RoomConfig, RoomStatus},
    },
    state::AppState,
};
use chrono::{Duration, Utc};
use std::collections::HashMap;

pub async fn create_room(
    state: AppState,
    name: Option<String>,
    room_config: Option<RoomConfig>,
) -> u32 {
    let mut rooms = state.rooms.lock().await;
    let new_id = rooms
        .keys()
        .map(|k| k.parse::<u32>().unwrap())
        .max()
        .unwrap_or(0)
        + 1;
    let new_room = Room::new(new_id.to_string(), name, room_config);
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
            room.empty_since = None;
            return true; // 再接続成功
        }

        // 新しいプレイヤーを追加
        let player = Player {
            id: player_id.to_string(),
            name: player_name.to_string(),
            // role: None,
            is_dead: false,
            is_ready: false,
        };
        room.players.push(player);
        room.empty_since = None;
        true
    } else {
        false
    }
}

pub async fn leave_room(state: AppState, room_id: &str, player_id: &str) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        if room.status == RoomStatus::InProgress {
            return Err("ゲーム進行中は退室できません。".to_string());
        }

        // プレイヤーが存在するかチェック
        let player_index = room.players.iter().position(|p| p.id == player_id);

        if let Some(index) = player_index {
            // プレイヤーを削除
            room.players.remove(index);

            if room.players.is_empty() {
                room.empty_since = Some(Utc::now());
                room.status = RoomStatus::Open;
            } else if room.status == RoomStatus::Ready {
                room.status = RoomStatus::Open;
            }

            Ok("Successfully left room".to_string())
        } else {
            Err("プレイヤーが見つかりません。".to_string())
        }
    } else {
        Err("ルームが見つかりません。".to_string())
    }
}

pub async fn get_rooms(state: &AppState) -> HashMap<String, Room> {
    state.rooms.lock().await.clone()
}

pub async fn get_room_info(state: &AppState, room_id: &str) -> Option<Room> {
    let rooms = state.rooms.lock().await;
    rooms.get(room_id).cloned()
}

pub async fn delete_room(state: AppState, room_id: &str, requester_id: &str) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;

    let Some(room) = rooms.get(room_id) else {
        return Err("ルームが見つかりません。".to_string());
    };

    if room.status == RoomStatus::InProgress {
        return Err("ゲーム進行中はルームを削除できません。".to_string());
    }

    let can_delete = room.players.iter().any(|player| player.id == requester_id);
    if !can_delete {
        return Err("ルーム参加者のみ削除できます。".to_string());
    }

    rooms.remove(room_id);
    Ok(format!("Room {} deleted successfully", room_id))
}

pub async fn cleanup_empty_rooms(state: &AppState, empty_room_ttl: Duration) -> usize {
    let mut rooms = state.rooms.lock().await;
    let now = Utc::now();
    let before_count = rooms.len();

    rooms.retain(|_, room| {
        if room.players.is_empty() {
            let empty_since = room.empty_since.unwrap_or(room.created_at);
            let elapsed = now.signed_duration_since(empty_since);
            return elapsed < empty_room_ttl;
        }
        true
    });

    before_count.saturating_sub(rooms.len())
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
                room.status = RoomStatus::Open;
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
    // tests are currently commented out.

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
