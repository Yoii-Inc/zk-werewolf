use std::collections::HashMap;

use crate::{
    models::{
        player::Player, role::Role, room::{Room, RoomStatus, Vote, VotingStatus}
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
            role: Role::Villager,
            is_dead: false,
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

// 投票開始
pub async fn start_voting(state: AppState, room_id: &str) -> bool {
    let mut rooms = state.rooms.lock().await;
    
    if let Some(room) = rooms.get_mut(room_id) {
        if room.voting_status == VotingStatus::NotStarted {
            room.start_voting();
            true
        } else {
            false
        }
    } else {
        false
    }
}

// 投票実行
pub async fn cast_vote(state: AppState, room_id: &str, voter_id: u32, target_id: u32) -> bool {
    let mut rooms = state.rooms.lock().await;
    
    if let Some(room) = rooms.get_mut(room_id) {
        // 投票が進行中か確認
        if room.voting_status != VotingStatus::InProgress {
            return false;
        }

        // 投票者と対象が存在するか確認
        if !room.players.iter().any(|p| p.id == voter_id) || 
           !room.players.iter().any(|p| p.id == target_id) {
            return false;
        }

        // 既に投票済みかチェック
        if room.votes.values().any(|v| v.voters.contains(&voter_id)) {
            return false;
        }

        // 投票を記録
        room.votes
            .entry(target_id)
            .or_insert_with(|| Vote {
                target_id,
                voters: Vec::new(),
            })
            .voters
            .push(voter_id);

        true
    } else {
        false
    }
}

// 投票終了
pub async fn end_voting(state: AppState, room_id: &str) -> Option<u32> {
    let mut rooms = state.rooms.lock().await;
    
    if let Some(room) = rooms.get_mut(room_id) {
        if room.voting_status == VotingStatus::InProgress {
            room.end_voting();
            
            // 最多得票者を取得
            room.votes
                .iter()
                .max_by_key(|(_, vote)| vote.voters.len())
                .map(|(target_id, _)| *target_id)
        } else {
            None
        }
    } else {
        None
    }
}

// 投票リセット
pub async fn reset_voting(state: AppState, room_id: &str) -> bool {
    let mut rooms = state.rooms.lock().await;
    
    if let Some(room) = rooms.get_mut(room_id) {
        room.reset_voting();
        true
    } else {
        false
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
