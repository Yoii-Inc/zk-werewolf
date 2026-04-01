use crate::{
    models::{
        game::{Game, GamePhase},
        player::Player,
        room::{Room, RoomConfig, RoomStatus},
    },
    state::AppState,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct RoomCleanupPolicy {
    pub room_empty_ttl: Duration,
    pub game_finished_ttl: Duration,
    pub game_stalled_ttl: Duration,
    pub game_max_day_count: u32,
}

impl Default for RoomCleanupPolicy {
    fn default() -> Self {
        Self {
            room_empty_ttl: Duration::minutes(10),
            game_finished_ttl: Duration::minutes(15),
            game_stalled_ttl: Duration::minutes(60),
            game_max_day_count: 20,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RoomCleanupResult {
    pub removed_rooms: usize,
    pub removed_games: usize,
    pub removed_channels: usize,
    pub removed_event_stores: usize,
    pub removed_proof_jobs: usize,
}

impl RoomCleanupResult {
    fn merge(&mut self, other: Self) {
        self.removed_rooms += other.removed_rooms;
        self.removed_games += other.removed_games;
        self.removed_channels += other.removed_channels;
        self.removed_event_stores += other.removed_event_stores;
        self.removed_proof_jobs += other.removed_proof_jobs;
    }
}

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

pub async fn delete_room(
    state: AppState,
    room_id: &str,
    requester_id: &str,
) -> Result<String, String> {
    {
        let rooms = state.rooms.lock().await;

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
    }

    delete_room_fully(&state, room_id).await;
    Ok(format!("Room {} deleted successfully", room_id))
}

pub async fn cleanup_rooms_and_games(
    state: &AppState,
    policy: RoomCleanupPolicy,
) -> RoomCleanupResult {
    let now = Utc::now();
    let room_ids_to_remove = {
        let rooms = state.rooms.lock().await;
        let games = state.games.lock().await;
        let mut room_ids = Vec::new();

        for (room_id, room) in rooms.iter() {
            let mut should_cleanup = should_cleanup_empty_room(room, now, &policy);
            if let Some(game) = games.get(room_id) {
                should_cleanup = should_cleanup
                    || should_cleanup_stalled_game(Some(room), game, now, &policy)
                    || should_cleanup_finished_game(Some(room), game, now, &policy);
            }

            if should_cleanup {
                room_ids.push(room_id.clone());
            }
        }

        for (room_id, game) in games.iter() {
            if rooms.contains_key(room_id) {
                continue;
            }

            if should_cleanup_stalled_game(None, game, now, &policy)
                || should_cleanup_finished_game(None, game, now, &policy)
            {
                room_ids.push(room_id.clone());
            }
        }

        room_ids.sort();
        room_ids.dedup();
        room_ids
    };

    let mut result = RoomCleanupResult::default();
    for room_id in room_ids_to_remove {
        let removed = delete_room_fully(state, &room_id).await;
        result.merge(removed);
    }

    result
}

async fn delete_room_fully(state: &AppState, room_id: &str) -> RoomCleanupResult {
    let room_removed = {
        let mut rooms = state.rooms.lock().await;
        rooms.remove(room_id).is_some()
    };
    let game_removed = {
        let mut games = state.games.lock().await;
        games.remove(room_id).is_some()
    };
    let (channel_removed, event_store_removed) = state.remove_room_runtime_resources(room_id).await;
    let removed_proof_jobs = state.proof_job_service.remove_room_jobs(room_id).await;

    RoomCleanupResult {
        removed_rooms: usize::from(room_removed),
        removed_games: usize::from(game_removed),
        removed_channels: usize::from(channel_removed),
        removed_event_stores: usize::from(event_store_removed),
        removed_proof_jobs,
    }
}

fn should_cleanup_empty_room(room: &Room, now: DateTime<Utc>, policy: &RoomCleanupPolicy) -> bool {
    if !room.players.is_empty() {
        return false;
    }
    let empty_since = room.empty_since.unwrap_or(room.created_at);
    now.signed_duration_since(empty_since) >= policy.room_empty_ttl
}

fn should_cleanup_stalled_game(
    room: Option<&Room>,
    game: &Game,
    now: DateTime<Utc>,
    policy: &RoomCleanupPolicy,
) -> bool {
    if game.phase == GamePhase::Finished {
        return false;
    }

    let in_progress = room
        .map(|r| r.status == RoomStatus::InProgress)
        .unwrap_or(true);
    if !in_progress {
        return false;
    }

    if game.day_count >= policy.game_max_day_count {
        return true;
    }

    now.signed_duration_since(game.phase_started_at) >= policy.game_stalled_ttl
}

fn should_cleanup_finished_game(
    room: Option<&Room>,
    game: &Game,
    now: DateTime<Utc>,
    policy: &RoomCleanupPolicy,
) -> bool {
    let is_finished = game.phase == GamePhase::Finished
        || room
            .map(|r| r.status == RoomStatus::Closed)
            .unwrap_or(false);
    if !is_finished {
        return false;
    }

    let finished_at = game.ended_at.unwrap_or(game.phase_started_at);
    now.signed_duration_since(finished_at) >= policy.game_finished_ttl
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
    use super::*;
    use crate::{
        models::{
            game::{BatchKey, CircuitProfileKey, ProofTypeKey},
            room::RoomConfig,
        },
        services::proof_job_service::{NodeJobStatus, ProofJobStatus},
        utils::test_setup::setup_test_env,
    };
    use mpc_algebra_wasm::{GroupingParameter, Role as GroupingRole};
    use serde_json::json;
    use std::collections::{BTreeMap, HashMap};

    fn make_grouping_parameter() -> GroupingParameter {
        let mut map = BTreeMap::new();
        map.insert(GroupingRole::FortuneTeller, (1, false));
        map.insert(GroupingRole::Werewolf, (1, false));
        map.insert(GroupingRole::Villager, (2, false));
        GroupingParameter::new(map)
    }

    fn make_players() -> Vec<Player> {
        vec![
            Player {
                id: "p1".to_string(),
                name: "p1".to_string(),
                is_dead: false,
                is_ready: true,
            },
            Player {
                id: "p2".to_string(),
                name: "p2".to_string(),
                is_dead: false,
                is_ready: true,
            },
            Player {
                id: "p3".to_string(),
                name: "p3".to_string(),
                is_dead: false,
                is_ready: true,
            },
            Player {
                id: "p4".to_string(),
                name: "p4".to_string(),
                is_dead: false,
                is_ready: true,
            },
        ]
    }

    fn make_room_and_game(room_id: &str) -> (Room, Game) {
        let players = make_players();
        let mut room = Room::new(
            room_id.to_string(),
            Some("test-room".to_string()),
            Some(RoomConfig::default()),
        );
        room.players = players.clone();
        room.status = RoomStatus::InProgress;
        room.empty_since = None;

        let mut game = Game::new(
            room_id.to_string(),
            players,
            room.max_players,
            make_grouping_parameter(),
        );
        game.phase = GamePhase::Night;
        game.phase_started_at = Utc::now();
        game.day_count = 1;
        (room, game)
    }

    async fn insert_room_and_game(state: &AppState, room: Room, game: Game) {
        state.rooms.lock().await.insert(room.room_id.clone(), room);
        state.games.lock().await.insert(game.room_id.clone(), game);
    }

    async fn seed_runtime_resources(state: &AppState, room_id: &str) {
        let tx = state.get_or_create_room_channel(room_id).await;
        let _rx = tx.subscribe();
        state
            .publish_room_event(room_id, json!({ "message_type": "test_event" }))
            .await
            .unwrap();

        let status = ProofJobStatus {
            state: "pending".to_string(),
            batch_id: format!("batch-{}", room_id),
            room_id: room_id.to_string(),
            batch_key: BatchKey {
                room_id: room_id.to_string(),
                phase: GamePhase::Night,
                day_count: 1,
                proof_type: ProofTypeKey::RoleAssignment,
                circuit_profile: CircuitProfileKey {
                    player_count: 4,
                    werewolf_count: 1,
                },
            },
            job_node_status: HashMap::from([(
                "node-0".to_string(),
                NodeJobStatus {
                    state: "pending".to_string(),
                    attempt_count: 0,
                    last_error: None,
                },
            )]),
            attempt_count: 0,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.proof_job_service.insert_status_for_test(status).await;
    }

    #[tokio::test]
    async fn cleanup_removes_stalled_game_by_day_count_and_related_state() {
        setup_test_env();
        let state = AppState::new();
        let room_id = "room-day-count";
        let (room, mut game) = make_room_and_game(room_id);
        game.day_count = 20;
        insert_room_and_game(&state, room, game).await;
        seed_runtime_resources(&state, room_id).await;

        let result = cleanup_rooms_and_games(&state, RoomCleanupPolicy::default()).await;

        assert_eq!(result.removed_rooms, 1);
        assert_eq!(result.removed_games, 1);
        assert_eq!(result.removed_channels, 1);
        assert_eq!(result.removed_event_stores, 1);
        assert_eq!(result.removed_proof_jobs, 1);
        assert!(!state.rooms.lock().await.contains_key(room_id));
        assert!(!state.games.lock().await.contains_key(room_id));
        assert!(!state.has_room_channel_for_test(room_id).await);
        assert!(!state.has_room_event_store_for_test(room_id).await);
        assert_eq!(
            state
                .proof_job_service
                .count_statuses_for_room_for_test(room_id)
                .await,
            0
        );
    }

    #[tokio::test]
    async fn cleanup_removes_stalled_game_by_phase_elapsed_time() {
        setup_test_env();
        let state = AppState::new();
        let room_id = "room-stalled";
        let (room, mut game) = make_room_and_game(room_id);
        game.day_count = 2;
        game.phase_started_at = Utc::now() - Duration::minutes(61);
        insert_room_and_game(&state, room, game).await;

        let result = cleanup_rooms_and_games(&state, RoomCleanupPolicy::default()).await;

        assert_eq!(result.removed_rooms, 1);
        assert!(!state.rooms.lock().await.contains_key(room_id));
        assert!(!state.games.lock().await.contains_key(room_id));
    }

    #[tokio::test]
    async fn cleanup_removes_finished_game_after_ttl() {
        setup_test_env();
        let state = AppState::new();
        let room_id = "room-finished";
        let (mut room, mut game) = make_room_and_game(room_id);
        room.status = RoomStatus::Closed;
        game.phase = GamePhase::Finished;
        game.ended_at = Some(Utc::now() - Duration::minutes(16));
        insert_room_and_game(&state, room, game).await;

        let result = cleanup_rooms_and_games(&state, RoomCleanupPolicy::default()).await;

        assert_eq!(result.removed_rooms, 1);
        assert!(!state.rooms.lock().await.contains_key(room_id));
        assert!(!state.games.lock().await.contains_key(room_id));
    }

    #[tokio::test]
    async fn cleanup_keeps_non_target_in_progress_games() {
        setup_test_env();
        let state = AppState::new();
        let room_id = "room-keep";
        let (room, mut game) = make_room_and_game(room_id);
        game.day_count = 5;
        game.phase_started_at = Utc::now() - Duration::minutes(30);
        insert_room_and_game(&state, room, game).await;

        let result = cleanup_rooms_and_games(&state, RoomCleanupPolicy::default()).await;

        assert_eq!(result.removed_rooms, 0);
        assert!(state.rooms.lock().await.contains_key(room_id));
        assert!(state.games.lock().await.contains_key(room_id));
    }

    #[tokio::test]
    async fn cleanup_removes_only_expired_empty_rooms() {
        setup_test_env();
        let state = AppState::new();

        let mut expired_room = Room::new("expired".to_string(), None, None);
        expired_room.empty_since = Some(Utc::now() - Duration::minutes(11));
        state
            .rooms
            .lock()
            .await
            .insert("expired".to_string(), expired_room);

        let mut active_room = Room::new("active".to_string(), None, None);
        active_room.empty_since = Some(Utc::now() - Duration::minutes(5));
        state
            .rooms
            .lock()
            .await
            .insert("active".to_string(), active_room);

        let result = cleanup_rooms_and_games(&state, RoomCleanupPolicy::default()).await;

        assert_eq!(result.removed_rooms, 1);
        assert!(!state.rooms.lock().await.contains_key("expired"));
        assert!(state.rooms.lock().await.contains_key("active"));
    }
}
