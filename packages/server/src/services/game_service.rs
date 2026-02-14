use crate::{
    blockchain::state_hash::{compute_game_id, compute_game_state_hash, is_evm_address},
    models::{
        chat::{ChatMessage, ChatMessageType},
        game::{Game, GamePhase, GameResult, NightAction, NightActionRequest},
        role::Role,
        room::{RoleConfig, RoomStatus},
    },
    services::zk_proof::{check_proof_status, request_proof_with_output},
    state::AppState,
};
use ark_bn254::Fr;
use ark_crypto_primitives::{encryption::AsymmetricEncryptionScheme, CommitmentScheme};
use ark_ff::UniformRand;
use rand::seq::SliceRandom;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use zk_mpc::{
    circuits::{ElGamalLocalOrMPC, KeyPublicizeCircuit, LocalOrMPC},
    input::{MpcInputTrait, WerewolfKeyInput, WerewolfMpcInput},
    marlin::MFr,
};
use mpc_algebra_wasm::{GroupingParameter, Role as GroupingRole};

fn grouping_parameter_from_role_config(role_config: &RoleConfig) -> GroupingParameter {
    let mut map = BTreeMap::new();
    map.insert(GroupingRole::FortuneTeller, (role_config.seer, false));
    map.insert(
        GroupingRole::Werewolf,
        (role_config.werewolf, role_config.werewolf > 1),
    );
    map.insert(GroupingRole::Villager, (role_config.villager, false));
    GroupingParameter::new(map)
}

// ゲームのライフサイクル管理
pub async fn start_game(state: AppState, room_id: &str) -> Result<String, String> {
    let game_snapshot = {
        let mut rooms = state.rooms.lock().await;
        let room = rooms.get_mut(room_id).ok_or("Room not found".to_string())?;

        // プレイヤー数に応じて役職を振り分け（デバッグ用に生成のみ）
        let _roles = assign_roles(room.players.len())?;
        let joined_players = room.players.len();
        let mut effective_role_config = room.room_config.role_config.clone();
        if joined_players < effective_role_config.seer + effective_role_config.werewolf {
            return Err(format!(
                "joined players ({}) are fewer than required special roles (seer + werewolf = {})",
                joined_players,
                effective_role_config.seer + effective_role_config.werewolf
            ));
        }

        if effective_role_config.total_players() != joined_players {
            effective_role_config.villager =
                joined_players.saturating_sub(effective_role_config.seer + effective_role_config.werewolf);
        }

        let grouping_parameter = grouping_parameter_from_role_config(&effective_role_config);
        let mut new_game = Game::new(
            room_id.to_string(),
            room.players.clone(),
            room.room_config.max_players,
            grouping_parameter,
        );

        // 暗号パラメータの初期化
        initialize_crypto_parameters(&mut new_game);

        let mut games = state.games.lock().await;
        let game = games.entry(room_id.to_string()).or_insert(new_game);

        room.status = RoomStatus::InProgress;

        let player_count = game.players.len();
        let start_message = format!(
            "Starting the game with {} players. Roles will be assigned via MPC.",
            player_count
        );
        game.chat_log.add_system_message(start_message);
        game.clone()
    };

    persist_game_on_chain(&state, &game_snapshot).await;

    // ゲーム開始後、最初のフェーズに進める
    advance_game_phase(state.clone(), room_id).await?;
    Ok("Game started successfully".to_string())
}

pub async fn end_game(state: AppState, room_id: String) -> Result<String, String> {
    let game_snapshot = {
        let mut rooms = state.rooms.lock().await;
        let mut games = state.games.lock().await;

        let room = rooms
            .get_mut(&room_id)
            .ok_or("Game not found".to_string())?;
        let game = games
            .get_mut(&room_id)
            .ok_or("Game not found".to_string())?;

        room.status = RoomStatus::Closed;
        game.phase = GamePhase::Finished;
        game.clone()
    };

    finalize_game_on_chain(&state, &game_snapshot).await;
    Ok("Game ended successfully".to_string())
}

pub async fn get_game_state(state: AppState, room_id: String) -> Result<Game, String> {
    let games = state.games.lock().await;
    // if let Some(game) = games.get(&room_id) {
    //     Ok(game.to_string())
    // } else {
    //     Err("Game not found".to_string())
    // }
    games
        .get(&room_id)
        .cloned()
        .ok_or("Game not found".to_string())
}

// ゲームフェーズ管理
pub async fn advance_game_phase(state: AppState, room_id: &str) -> Result<String, String> {
    let current_phase = {
        let games = state.games.lock().await;
        if let Some(game) = games.get(room_id) {
            game.phase.clone()
        } else {
            return Err("Game not found".to_string());
        }
    };

    let next_phase = match current_phase {
        GamePhase::Waiting => GamePhase::Night,
        GamePhase::Night => GamePhase::DivinationProcessing,
        GamePhase::DivinationProcessing => GamePhase::Discussion,
        GamePhase::Discussion => GamePhase::Voting,
        GamePhase::Voting => GamePhase::Result,
        GamePhase::Result => GamePhase::Night,
        GamePhase::Finished => return Err("ゲームは既に終了しています".to_string()),
    };

    let (from_phase_str, to_phase_str, game_snapshot) = {
        let mut games = state.games.lock().await;
        let game = games.get_mut(room_id).ok_or("Game not found".to_string())?;

        if current_phase == GamePhase::Night {
            game.resolve_night_actions();
        } else if current_phase == GamePhase::Voting {
            game.resolve_voting();
        } else if current_phase == GamePhase::Result {
            game.advance_to_next_day();
        }

        game.add_phase_change_message(current_phase.clone(), next_phase.clone());
        game.phase = next_phase.clone();

        (
            format!("{:?}", current_phase),
            format!("{:?}", next_phase),
            game.clone(),
        )
    };

    if let Err(e) = state
        .broadcast_phase_change(room_id, &from_phase_str, &to_phase_str)
        .await
    {
        eprintln!("Failed to broadcast phase change: {}", e);
    }

    update_game_state_on_chain(&state, &game_snapshot).await;
    Ok(format!("フェーズを更新しました: {:?}", next_phase))
}

async fn persist_game_on_chain(state: &AppState, game: &Game) {
    if !state.blockchain_client.is_enabled() {
        return;
    }

    let game_id = compute_game_id(&game.room_id);
    let players = extract_evm_player_addresses(game);
    if players.is_empty() {
        tracing::warn!(
            "Skipping on-chain create_game for room {}: no EVM-style player IDs found",
            game.room_id
        );
        return;
    }

    if let Err(e) = state.blockchain_client.create_game(game_id, players).await {
        tracing::error!("Failed to create game on-chain: {}", e);
        return;
    }

    update_game_state_on_chain(state, game).await;
}

pub async fn update_game_state_on_chain(state: &AppState, game: &Game) {
    if !state.blockchain_client.is_enabled() {
        return;
    }

    let game_id = compute_game_id(&game.room_id);
    let state_hash = compute_game_state_hash(game);
    if let Err(e) = state
        .blockchain_client
        .update_game_state(game_id, state_hash)
        .await
    {
        tracing::error!("Failed to update game state on-chain: {}", e);
    }
}

async fn finalize_game_on_chain(state: &AppState, game: &Game) {
    if !state.blockchain_client.is_enabled() {
        return;
    }

    let game_id = compute_game_id(&game.room_id);
    let state_hash = compute_game_state_hash(game);

    if let Err(e) = state
        .blockchain_client
        .update_game_state(game_id, state_hash)
        .await
    {
        tracing::error!("Failed to update final game state on-chain: {}", e);
    }

    if let Err(e) = state
        .blockchain_client
        .finalize_game(game_id, game.result.clone())
        .await
    {
        tracing::error!("Failed to finalize game on-chain: {}", e);
    }

    let winners = extract_winner_addresses(game);
    if !winners.is_empty() {
        if let Err(e) = state
            .blockchain_client
            .distribute_rewards(game_id, winners)
            .await
        {
            tracing::error!("Failed to distribute rewards on-chain: {}", e);
        }
    }
}

fn extract_evm_player_addresses(game: &Game) -> Vec<String> {
    game.players
        .iter()
        .filter_map(|player| {
            if is_evm_address(&player.id) {
                Some(player.id.clone())
            } else {
                None
            }
        })
        .collect()
}

fn extract_winner_addresses(game: &Game) -> Vec<String> {
    // サーバー側では役職を保持していないため、結果判定後に生存者全員を暫定 winners として扱う。
    game.players
        .iter()
        .filter(|player| !player.is_dead && is_evm_address(&player.id))
        .map(|player| player.id.clone())
        .collect()
}

// 夜のアクション処理
pub async fn process_night_action(
    state: AppState,
    room_id: &str,
    action_req: NightActionRequest,
) -> Result<String, String> {
    let games = state.games.lock().await;
    let game = games.get(room_id).ok_or("Game not found")?;

    if game.phase != GamePhase::Night {
        return Err("夜のアクションは夜にのみ実行できます".to_string());
    }

    // プレイヤーの存在確認のみ実施（役職チェックは行わない）
    let _player = game
        .players
        .iter()
        .find(|p| p.id == action_req.player_id)
        .ok_or("プレイヤーが見つかりません")?;

    // 役職情報はクライアント側（MPC計算結果）で管理されるため、
    // サーバーは役職チェックを行わず、アクションタイプに応じた処理のみ実施
    match &action_req.action {
        NightAction::Attack { target_id } => {
            // 人狼の襲撃処理
            drop(games);
            let mut games = state.games.lock().await;
            let game = games.get_mut(room_id).ok_or("Game not found")?;
            game.register_attack(target_id)?;
            Ok("襲撃先を登録しました".to_string())
        }
    }
}

async fn check_status_with_retry(proof_id: &str) -> Result<bool, String> {
    for _ in 0..30 {
        if check_proof_status(proof_id).await?.0 {
            return Ok(true);
        }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(false)
}

// // 勝利判定
// pub async fn check_winner(state: AppState, room_id: &str) -> Result<GameResult, String> {
//     let games = state.games.lock().await;
//     let game = games.get(room_id).ok_or("Game not found")?;

//     // 生存者のカウント
//     let living_players: Vec<_> = game.players.iter().filter(|p| !p.is_dead).collect();
//     let alive_villagers = living_players
//         .iter()
//         .filter(|p| p.role.as_ref() != Some(&Role::Werewolf))
//         .count();
//     let alive_werewolves = living_players
//         .iter()
//         .filter(|p| p.role.as_ref() == Some(&Role::Werewolf))
//         .count();

//     if state.debug_config.create_proof {
//         // 勝利判定の証明
//         let winner_inputs = json!({
//             "alive_villagers": alive_villagers,
//             "alive_werewolves": alive_werewolves,
//             "total_players": game.players.len(),
//             "is_game_in_progress": game.phase != GamePhase::Finished
//         });

//         let winning_judge_circuit = zk_mpc::circuits::WinningJudgeCircuit::<MFr> {
//             num_alive: todo!(),
//             pedersen_param: todo!(),
//             am_werewolf: todo!(),
//             game_state: todo!(),
//             player_randomness: todo!(),
//             player_commitment: todo!(),
//         };

//         todo!();

//         // let proof_id = request_proof_with_output(
//         //     CircuitIdentifier::Built(BuiltinCircuit::WinningJudge(winning_judge_circuit)),
//         //     ProofOutputType::Public,
//         // )
//         // .await?;

//         // if check_status_with_retry(&proof_id).await? {
//         //     let result = if alive_werewolves == 0 {
//         //         GameResult::VillagerWin
//         //     } else if alive_werewolves >= alive_villagers {
//         //         GameResult::WerewolfWin
//         //     } else {
//         //         GameResult::InProgress
//         //     };

//         //     drop(games);
//         //     let mut games = state.games.lock().await;
//         //     let game = games.get_mut(room_id).ok_or("Game not found")?;
//         //     game.result = result.clone();
//         //     Ok(result)
//         // } else {
//         //     Err("勝利判定の証明に失敗しました".to_string())
//         // }
//     } else {
//         let result = if alive_werewolves == 0 {
//             GameResult::VillagerWin
//         } else if alive_werewolves >= alive_villagers {
//             GameResult::WerewolfWin
//         } else {
//             GameResult::InProgress
//         };

//         drop(games);
//         let mut games = state.games.lock().await;
//         let game = games.get_mut(room_id).ok_or("Game not found")?;

//         if result != GameResult::InProgress {
//             let (winner_message, details) = match result {
//                 GameResult::VillagerWin => (
//                     "Villagers team wins!",
//                     format!("Remaining villagers: {}", alive_villagers),
//                 ),
//                 GameResult::WerewolfWin => (
//                     "Werewolves team wins!",
//                     format!(
//                         "Remaining werewolves: {}, Remaining villagers: {}",
//                         alive_werewolves, alive_villagers
//                     ),
//                 ),
//                 GameResult::InProgress => unreachable!(),
//             };

//             game.chat_log.add_message(ChatMessage::new(
//                 "system".to_string(),
//                 "System".to_string(),
//                 format!("{} {}", winner_message, details),
//                 ChatMessageType::System,
//             ));

//             game.phase = GamePhase::Finished;
//         }

//         game.result = result.clone();
//         Ok(result)
//     }
// }

// 役職の振り分け
pub fn assign_roles(players_count: usize) -> Result<Vec<Role>, String> {
    if players_count < 4 {
        return Err("At least 4 players are required".to_string());
    }

    let mut roles = Vec::new();

    // 人狼の数を決定（プレイヤー数に応じて）
    let werewolf_count = match players_count {
        4..=6 => 1,
        7..=9 => 2,
        10..=15 => 3,
        _ => (players_count as f32 * 0.3).ceil() as usize, // 30%ルール
    };

    // 占い師は1人
    let seer_count = 1;

    // 残りは村人
    let villager_count = players_count - werewolf_count - seer_count;

    // 役職リストを作成
    roles.extend(vec![Role::Werewolf; werewolf_count]);
    roles.extend(vec![Role::Seer; seer_count]);
    roles.extend(vec![Role::Villager; villager_count]);

    // 役職をシャッフル
    let mut rng = rand::thread_rng();
    roles.shuffle(&mut rng);

    Ok(roles)
}

// 暗号パラメータの初期化関数
pub fn initialize_crypto_parameters(game: &mut Game) {
    let mut rng = rand::thread_rng();

    // Pedersenコミットメントパラメータの生成
    let pedersen_param =
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::setup(&mut rng).unwrap();

    // ElGamalパラメータと鍵ペアの生成
    let elgamal_param =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::setup(
            &mut rng,
        )
        .unwrap();
    let (pk, sk) =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::keygen(
            &elgamal_param,
            &mut rng,
        )
        .unwrap();

    // プレイヤーごとのコミットメント（空で初期化、後でクライアントから受信）
    let player_commitment: Vec<
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::Output,
    > = Vec::new();

    game.crypto_parameters = Some(crate::models::game::CryptoParameters {
        pedersen_param,
        player_commitment,
        fortune_teller_public_key: None,
        elgamal_param,
    });

    tracing::info!("Initialized crypto parameters for game {}", game.room_id);
}
