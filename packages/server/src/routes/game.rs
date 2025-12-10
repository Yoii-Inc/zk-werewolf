use crate::models::game::{
    BatchRequest, ChangeRoleRequest, ClientRequestType, ComputationResults, GamePhase, GameResult,
    NightActionRequest,
};
use crate::models::role::Role;
use crate::models::room::RoomStatus;
use crate::services::zk_proof;
use crate::{
    models::chat::{ChatMessage, ChatMessageType},
    services::game_service,
    state::AppState,
};
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteAction {
    // voter_id: String,
    // target_id: String,
    prover_num: String,
    encrypted_proof_input: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .nest(
            "/:roomid",
            Router::new()
                // ゲームの基本操作
                .route("/start", post(start_game))
                .route("/end", post(end_game_handler))
                // curl http://localhost:8080/api/game/{roomid}/state
                .route("/state", get(get_game_state))
                // ゲームアクション
                .nest(
                    "/actions",
                    Router::new()
                        .route("/vote", post(cast_vote_handler))
                        .route("/night-action", post(night_action_handler)),
                )
                .route("/proof", post(proof_handler))
                // 暗号パラメータとコミットメント管理
                .route("/crypto-params", get(get_crypto_params))
                .route("/commitment", post(submit_commitment))
                // デバッグ用エンドポイント
                .route("/debug/change-role", post(change_player_role))
                .route("/debug/reset", post(reset_game_handler))
                .route("/debug/reset-batch", post(reset_batch_request_handler))
                // ゲーム進行の管理
                .route("/phase/next", post(advance_phase_handler))
                .route("/check-winner", get(check_winner_handler))
                .route("/messages/:player_id", get(get_messages)),
        )
        .with_state(state)
}

pub async fn start_game(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::start_game(state, &room_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::NOT_FOUND, Json(message)),
    }
}

pub async fn get_game_state(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // match game_service::get_game_state(state, room_id).await {
    //     Ok(game) => (StatusCode::OK, Json(game)),
    //     Err(message) => (StatusCode::NOT_FOUND, Json(message)),
    // }
    let game = game_service::get_game_state(state, room_id).await.unwrap();
    (StatusCode::OK, Json(game))
    // Err(message) => (StatusCode::NOT_FOUND, Json(message)),
}

async fn end_game_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::end_game(state, room_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::NOT_FOUND, Json(message)),
    }
}

async fn night_action_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(action_req): Json<NightActionRequest>,
) -> impl IntoResponse {
    match game_service::process_night_action(state, &room_id, action_req).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(e)),
    }
}

async fn cast_vote_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(_vote_action): Json<VoteAction>,
) -> impl IntoResponse {
    match game_service::handle_vote(
        state,
        &room_id,
        // &vote_action.voter_id,
        // &vote_action.target_id,
        &String::from("1"), // voter_idは未使用のため空文字列を渡す
        &String::from("1"), // target_idは未使用のため空文字列を渡す
    )
    .await
    {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::BAD_REQUEST, Json(message)),
    }
}

pub async fn proof_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(request): Json<ClientRequestType>,
) -> impl IntoResponse {
    match zk_proof::batch_proof_handling(state, &room_id, &request).await {
        Ok(batch_id) => (StatusCode::OK, Json(batch_id)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e)),
    }
}

async fn advance_phase_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::advance_game_phase(state, &room_id).await {
        Ok(message) => (StatusCode::OK, Json(message)),
        Err(message) => (StatusCode::BAD_REQUEST, Json(message)),
    }
}

async fn check_winner_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    match game_service::check_winner(state, &room_id).await {
        Ok(winner) => match winner {
            GameResult::InProgress => (StatusCode::OK, Json("Game in progress".to_string())),
            GameResult::VillagerWin => (StatusCode::OK, Json("Villagers team wins".to_string())),
            GameResult::WerewolfWin => (StatusCode::OK, Json("Werewolves team wins".to_string())),
        },
        Err(message) => (StatusCode::BAD_REQUEST, Json(message)),
    }
}

pub async fn change_player_role(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<ChangeRoleRequest>,
) -> impl IntoResponse {
    let mut games = state.games.lock().await;

    if let Some(game) = games.get_mut(&room_id) {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == payload.player_id) {
            // 文字列から Role を変換
            let new_role = match payload.new_role.as_str() {
                "Villager" => Some(Role::Villager),
                "Werewolf" => Some(Role::Werewolf),
                "Seer" => Some(Role::Seer),
                _ => None,
            };

            player.role = new_role;

            game.chat_log.add_system_message(format!(
                "{}の役職が{}に変更されました",
                player.name, payload.new_role
            ));

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "役職を変更しました"
                })),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "プレイヤーが見つかりません"
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "ゲームが見つかりません"
            })),
        )
    }
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path((room_id, player_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let (rooms, games) = tokio::join!(state.rooms.lock(), state.games.lock());

    // playerを取得
    let player = if let Some(room) = rooms.get(&room_id) {
        room.players.iter().find(|p| p.id == player_id)
    } else if let Some(game) = games.get(&room_id) {
        game.players.iter().find(|p| p.id == player_id)
    } else {
        None
    };

    if player.is_none() {
        return (StatusCode::NOT_FOUND, Json(Vec::<ChatMessage>::new()));
    }

    // ゲームが存在する場合はゲームのチャットログを返す
    if let Some(game) = games.get(&room_id) {
        let filtered_messages = game
            .chat_log
            .messages
            .iter()
            .filter(|msg| match msg.message_type {
                ChatMessageType::Wolf => player.unwrap().role == Some(Role::Werewolf),
                ChatMessageType::Private => msg.player_id == player_id,
                _ => true,
            })
            .cloned()
            .collect::<Vec<_>>();

        return (StatusCode::OK, Json(filtered_messages));
    }

    // ゲームが存在しない場合はルームのチャットログを返す
    if let Some(room) = rooms.get(&room_id) {
        return (StatusCode::OK, Json(room.chat_log.messages.clone()));
    }

    // どちらも存在しない場合は404を返す
    (StatusCode::NOT_FOUND, Json(Vec::<ChatMessage>::new()))
}

/// デバッグ用：ゲームをリセットして初期状態に戻す
async fn reset_game_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let mut games = state.games.lock().await;
    let mut rooms = state.rooms.lock().await;

    // ゲームが存在する場合、プレイヤー情報を保持したまま初期状態に戻す
    if let Some(game) = games.get_mut(&room_id) {
        let players = game.players.clone();

        // ゲームを初期状態に戻す
        let mut reset_game = game.clone();
        reset_game.phase = GamePhase::Waiting;
        reset_game.chat_log.messages.clear();
        // reset_game.has_acted.clear();

        // プレイヤーの状態をリセット
        for mut player in reset_game.players.iter_mut() {
            player.role = None;
            player.is_dead = false;
            // player.has_voted = false;
            // player.vote_count = 0;
        }

        // computation_results をリセット
        reset_game.computation_results = ComputationResults::default();

        // システムメッセージを追加
        reset_game
            .chat_log
            .add_system_message("Game has been reset".to_string());

        // ゲームを更新
        *game = reset_game;

        // ルームも更新
        if let Some(room) = rooms.get_mut(&room_id) {
            room.status = RoomStatus::Open;
            room.chat_log.messages.clear();
            room.chat_log
                .add_system_message("Room has been reset".to_string());
        }

        (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "ゲームをリセットしました"
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "ゲームが見つかりません"
            })),
        )
    }
}

/// デバッグ用：バッチリクエストをリセットする
async fn reset_batch_request_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let mut games = state.games.lock().await;

    if let Some(game) = games.get_mut(&room_id) {
        // バッチリクエストを新しいものに置き換え
        game.batch_request = BatchRequest::new();

        // システムメッセージを追加
        game.chat_log
            .add_system_message("Batch request has been reset".to_string());

        (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "バッチリクエストをリセットしました",
                "batch_id": game.batch_request.batch_id
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "ゲームが見つかりません"
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::game::BatchStatus, utils::test_setup::setup_test_env};
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_start_game() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());
        let room_id = crate::services::room_service::create_room(state.clone(), None).await;

        for i in 0..4 {
            crate::services::room_service::join_room(
                state.clone(),
                &room_id.to_string(),
                &format!("test_id_{}", i),
                &format!("test_player_{}", i),
            )
            .await;
        }

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/start", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_end_game() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());
        let room_id = crate::services::room_service::create_room(state.clone(), None).await;

        for i in 0..4 {
            crate::services::room_service::join_room(
                state.clone(),
                &room_id.to_string(),
                &format!("test_id_{}", i),
                &format!("test_player_{}", i),
            )
            .await;
        }

        game_service::start_game(state.clone(), &room_id.to_string())
            .await
            .unwrap();

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/end", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_reset_game() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());
        let room_id = crate::services::room_service::create_room(state.clone(), None).await;

        // プレイヤーを追加
        for i in 0..4 {
            crate::services::room_service::join_room(
                state.clone(),
                &room_id.to_string(),
                &format!("test_id_{}", i),
                &format!("test_player_{}", i),
            )
            .await;
        }

        // ゲームを開始
        game_service::start_game(state.clone(), &room_id.to_string())
            .await
            .unwrap();

        // ゲームをリセット
        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/debug/reset", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // ゲームの状態を確認
        let games = state.games.lock().await;
        let game = games.get(&room_id.to_string()).unwrap();
        assert_eq!(game.phase, GamePhase::Waiting);
        assert!(game.chat_log.messages.len() == 1); // リセットメッセージのみ

        // プレイヤーの状態を確認
        for player in &game.players {
            assert!(player.role.is_none());
            assert!(!player.is_dead);
            // assert!(!player.has_voted);
            // assert_eq!(player.vote_count, 0);
        }
    }

    #[tokio::test]
    async fn test_reset_batch_request() {
        setup_test_env();
        let state = AppState::new();
        let app = routes(state.clone());
        let room_id = crate::services::room_service::create_room(state.clone(), None).await;

        // プレイヤーを追加
        for i in 0..4 {
            crate::services::room_service::join_room(
                state.clone(),
                &room_id.to_string(),
                &format!("test_id_{}", i),
                &format!("test_player_{}", i),
            )
            .await;
        }

        // ゲームを開始
        game_service::start_game(state.clone(), &room_id.to_string())
            .await
            .unwrap();

        // バッチリクエストをリセット
        let request = Request::builder()
            .method("POST")
            .uri(&format!("/{}/debug/reset-batch", room_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // ゲームの状態を確認
        let games = state.games.lock().await;
        let game = games.get(&room_id.to_string()).unwrap();

        // バッチリクエストが新しく作成されていることを確認
        assert!(game.batch_request.requests.is_empty());
        assert_eq!(game.batch_request.status, BatchStatus::Collecting);
    }
}

// 暗号パラメータを取得
async fn get_crypto_params(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    // NOTE: このエンドポイントは現在使用されていません。
    // クライアント側では、暗号パラメータを静的ファイル(/public/*.json)から読み込んでいます。
    // 将来的に、ゲームごとの暗号パラメータをシリアライズして返す必要が生じた場合に実装予定です。
    // CryptoParametersのSerialize/Deserialize実装が完了したら、このエンドポイントで動的に返すことを検討してください。

    let games = state.games.lock().await;

    if let Some(game) = games.get(&room_id) {
        if let Some(_crypto_params) = &game.crypto_parameters {
            // CryptoParametersから必要な情報を抽出してJSONで返す
            // TODO: CryptoParametersのSerialize実装完了後に実装
            let params = json!({
                "pedersenParam": null, // crypto_params.pedersen_paramをシリアライズ
                "elgamalParam": null, // crypto_params.elgamal_paramをシリアライズ
                "playerCommitments": [], // crypto_params.player_commitmentをシリアライズ
                "gameId": room_id,
                "createdAt": game.started_at.map(|t| t.to_rfc3339()),
            });

            (StatusCode::OK, Json(params))
        } else {
            // 暗号パラメータがまだ生成されていない場合
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Crypto parameters not yet initialized"
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Game not found"
            })),
        )
    }
}

// コミットメントを受信
#[derive(Debug, Serialize, Deserialize)]
struct CommitmentRequest {
    player_id: i32,
    commitment: Vec<String>,
    created_at: String,
}

async fn submit_commitment(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(commitment_req): Json<CommitmentRequest>,
) -> impl IntoResponse {
    let mut games = state.games.lock().await;

    if let Some(game) = games.get_mut(&room_id) {
        if let Some(ref mut _crypto_params) = game.crypto_parameters {
            // TODO: commitment文字列をPedersenCommitment型にデシリアライズして保存
            // 現状はプレイヤーIDと対応づけてログに記録
            tracing::info!(
                "Received commitment from player {} in room {}: {:?}",
                commitment_req.player_id,
                room_id,
                commitment_req.commitment
            );

            // 将来的には以下のように保存:
            // let commitment_obj = deserialize_commitment(&commitment_req.commitment)?;
            // crypto_params.player_commitment.push(commitment_obj);

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Commitment received and logged (storage pending serialization implementation)"
                })),
            )
        } else {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": "Crypto parameters not initialized for this game"
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Game not found"
            })),
        )
    }
}
