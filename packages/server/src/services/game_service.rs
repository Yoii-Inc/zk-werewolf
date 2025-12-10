use crate::{
    models::{
        chat::{ChatMessage, ChatMessageType},
        game::{Game, GamePhase, GameResult, NightAction, NightActionRequest},
        role::Role,
        room::RoomStatus,
    },
    services::zk_proof::{check_proof_status, request_proof_with_output},
    state::AppState,
};
use ark_bls12_377::Fr;
use ark_crypto_primitives::{encryption::AsymmetricEncryptionScheme, CommitmentScheme};
use ark_ff::UniformRand;
use ark_std::test_rng;
use rand::seq::SliceRandom;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use zk_mpc::{
    circuits::{ElGamalLocalOrMPC, KeyPublicizeCircuit, LocalOrMPC},
    input::{MpcInputTrait, WerewolfKeyInput, WerewolfMpcInput},
    marlin::MFr,
};

// ゲームのライフサイクル管理
pub async fn start_game(state: AppState, room_id: &str) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        // プレイヤー数に応じて役職を振り分け
        let roles = assign_roles(room.players.len())?;
        let mut new_game = Game::new(room_id.to_string(), room.players.clone());

        // 暗号パラメータの初期化
        initialize_crypto_parameters(&mut new_game);

        let mut games = state.games.lock().await;
        let game = games.entry(room_id.to_string()).or_insert(new_game);

        // preprocessing_werewolf(state.clone(), &mut game).await;

        if state.debug_config.random_role {
            // 各プレイヤーに役職を割り当て
            for (player, role) in game.players.iter_mut().zip(roles.iter()) {
                player.role = Some(role.clone());
            }
        }

        room.status = RoomStatus::InProgress;

        // ゲーム開始のシステムメッセージを追加
        let player_count = game.players.len();
        let werewolf_count = game
            .players
            .iter()
            .filter(|p| p.role == Some(Role::Werewolf))
            .count();
        let seer_exists = game.players.iter().any(|p| p.role == Some(Role::Seer));

        let mut start_message = format!(
            "Starting the game. {} players have joined, including {} werewolves.",
            player_count, werewolf_count
        );
        if seer_exists {
            start_message.push_str(" The seer will also help protect the village.");
        }

        game.chat_log.add_system_message(start_message);

        drop(games);

        // ゲーム開始後、最初のフェーズに進める
        advance_game_phase(state.clone(), room_id).await?;

        Ok("Game started successfully".to_string())
    } else {
        Err("Room not found".to_string())
    }
}

pub async fn end_game(state: AppState, room_id: String) -> Result<String, String> {
    let mut rooms = state.rooms.lock().await;
    let mut games = state.games.lock().await;
    if let (Some(room), Some(game)) = (rooms.get_mut(&room_id), games.get_mut(&room_id)) {
        room.status = RoomStatus::Closed;
        game.phase = GamePhase::Finished;
        Ok("Game ended successfully".to_string())
    } else {
        Err("Game not found".to_string())
    }
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

    // フェーズ変更をゲームに適用

    let mut games = state.games.lock().await;
    if let Some(game) = games.get_mut(room_id) {
        if current_phase == GamePhase::Night {
            game.resolve_night_actions();
        } else if current_phase == GamePhase::Voting {
            game.resolve_voting();
        }
        game.add_phase_change_message(current_phase.clone(), next_phase.clone());
        game.phase = next_phase.clone();

        // WebSocket通知を送信
        let from_phase_str = format!("{:?}", current_phase);
        let to_phase_str = format!("{:?}", next_phase);

        if let Err(e) = state
            .broadcast_phase_change(room_id, &from_phase_str, &to_phase_str)
            .await
        {
            eprintln!("Failed to broadcast phase change: {}", e);
        }

        Ok(format!("フェーズを更新しました: {:?}", next_phase))
    } else {
        Err("Game not found".to_string())
    }
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

    let player = game
        .players
        .iter()
        .find(|p| p.id == action_req.player_id)
        .ok_or("プレイヤーが見つかりません")?;

    match (
        player.role.as_ref().unwrap_or(&Role::Villager),
        &action_req.action,
    ) {
        (Role::Werewolf, NightAction::Attack { target_id }) => {
            if state.debug_config.create_proof {
                // request proof of werewolf attack validity
                let _attack_inputs = json!({
                    "attacker_id": action_req.player_id,
                    "target_id": target_id,
                    "is_attacker_werewolf": true,
                    "is_night_phase": true,
                    "is_target_alive": !game.players.iter().find(|p| p.id == *target_id).map_or(true, |p| p.is_dead)
                });

                // TODO: Replace with actual circuit identifier

                todo!();

                // let proof_id = request_proof(
                //     zk_mpc_node::CircuitIdentifier::Built(WerewolfAttackCircuit),
                //     attack_inputs,
                // )
                // .await?;

                // if check_status_with_retry(&proof_id).await? {
                //     drop(games);
                //     let mut games = state.games.lock().await;
                //     let game = games.get_mut(room_id).ok_or("Game not found")?;
                //     game.register_attack(target_id)?;
                //     Ok("襲撃先を登録しました".to_string())
                // } else {
                //     Err("襲撃の証明に失敗しました".to_string())
                // }
            } else {
                drop(games);
                let mut games = state.games.lock().await;
                let game = games.get_mut(room_id).ok_or("Game not found")?;
                game.register_attack(target_id)?;
                Ok("襲撃先を登録しました".to_string())
            }
        }
        (Role::Seer, NightAction::Divine { target_id }) => {
            if state.debug_config.create_proof {
                // 占いの有効性を証明
                // let divine_inputs = json!({
                //     "seer_id": action_req.player_id,
                //     "target_id": target_id,
                //     "is_seer": true,
                //     "is_night_phase": true,
                //     "is_target_alive": !game.players.iter().find(|p| p.id == *target_id).map_or(true, |p| p.is_dead)
                // });

                let is_werewolf_vec = game
                    .players
                    .iter()
                    .map(|p| p.role == Some(Role::Werewolf))
                    .map(|b| Fr::from(b))
                    .collect::<Vec<_>>();
                let is_target_vec = game
                    .players
                    .iter()
                    .map(|p| p.id == *target_id)
                    .map(|b| Fr::from(b))
                    .collect::<Vec<_>>();

                let rng = &mut test_rng();

                // let (elgamal_param, elgamal_pubkey) = get_elgamal_param_pubkey();
                let elgamal_param = game
                    .crypto_parameters
                    .clone()
                    .unwrap()
                    .elgamal_param
                    .clone();
                let elgamal_pubkey = game
                    .crypto_parameters
                    .clone()
                    .unwrap()
                    .fortune_teller_public_key
                    .clone();

                let mut mpc_input = WerewolfMpcInput::init();
                mpc_input.set_public_input(rng, Some((elgamal_param, elgamal_pubkey)));
                mpc_input.set_private_input(Some((is_werewolf_vec.clone(), is_target_vec.clone())));
                mpc_input.generate_input(rng);

                let divination_circuit = zk_mpc::circuits::DivinationCircuit::<MFr> {
                    mpc_input: mpc_input.clone(),
                };

                // let proof_id = request_proof_with_output(
                //     CircuitIdentifier::Built(BuiltinCircuit::Divination(divination_circuit)),
                //     ProofOutputType::Public,
                // )
                // .await?;

                // if check_status_with_retry(&proof_id).await? {
                let result = game.divine_player(target_id)?;
                Ok(format!("プレイヤーの役職は {} です", result))
                // } else {
                // Err("占いの証明に失敗しました".to_string())
                // }
            } else {
                let result = game.divine_player(target_id)?;
                Ok(format!("プレイヤーの役職は {} です", result))
            }
        }
        _ => Err("このプレイヤーの役職ではこのアクションを実行できません".to_string()),
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

// 投票システム
pub async fn handle_vote(
    state: AppState,
    room_id: &str,
    voter_id: &str,
    target_id: &str,
) -> Result<String, String> {
    let games = state.games.lock().await;
    let game = games.get(room_id).ok_or("Game not found")?;

    if game.phase != GamePhase::Voting {
        return Err("現在は投票フェーズではありません".to_string());
    }

    if state.debug_config.create_proof {
        // 投票の有効性を証明
        let vote_inputs = json!({
            "voter_id": voter_id,
            "target_id": target_id,
            "is_voter_alive": !game.players.iter().find(|p| p.id == voter_id).map_or(true, |p| p.is_dead),
            "is_target_alive": !game.players.iter().find(|p| p.id == target_id).map_or(true, |p| p.is_dead),
            "is_voting_phase": game.phase == GamePhase::Voting
        });

        let anonymous_voting_circuit = zk_mpc::circuits::AnonymousVotingCircuit::<MFr> {
            is_target_id: todo!(),
            pedersen_param: todo!(),
            player_randomness: todo!(),
            player_commitment: todo!(),
        };

        todo!();

        // let proof_id = request_proof_with_output(
        //     CircuitIdentifier::Built(BuiltinCircuit::AnonymousVoting(anonymous_voting_circuit)),
        //     ProofOutputType::Public,
        // )
        // .await?;

        // // 証明の完了を待つ
        // for _ in 0..30 {
        //     if check_proof_status(&proof_id).await?.0 {
        //         drop(games); // 先のロックを解放
        //         let mut games = state.games.lock().await;
        //         let game = games.get_mut(room_id).ok_or("Game not found")?;
        //         game.cast_vote(voter_id, target_id)?;
        //         return Ok("投票を受け付けました".to_string());
        //     }
        //     sleep(Duration::from_secs(1)).await;
        // }

        Err("投票の証明に失敗しました".to_string())
    } else {
        drop(games); // 先のロックを解放
        let mut games = state.games.lock().await;
        let game = games.get_mut(room_id).ok_or("Game not found")?;
        game.cast_vote(voter_id, target_id)?;
        Ok("投票を受け付けました".to_string())
    }
}

// 勝利判定
pub async fn check_winner(state: AppState, room_id: &str) -> Result<GameResult, String> {
    let games = state.games.lock().await;
    let game = games.get(room_id).ok_or("Game not found")?;

    // 生存者のカウント
    let living_players: Vec<_> = game.players.iter().filter(|p| !p.is_dead).collect();
    let alive_villagers = living_players
        .iter()
        .filter(|p| p.role.as_ref() != Some(&Role::Werewolf))
        .count();
    let alive_werewolves = living_players
        .iter()
        .filter(|p| p.role.as_ref() == Some(&Role::Werewolf))
        .count();

    if state.debug_config.create_proof {
        // 勝利判定の証明
        let winner_inputs = json!({
            "alive_villagers": alive_villagers,
            "alive_werewolves": alive_werewolves,
            "total_players": game.players.len(),
            "is_game_in_progress": game.phase != GamePhase::Finished
        });

        let winning_judge_circuit = zk_mpc::circuits::WinningJudgeCircuit::<MFr> {
            num_alive: todo!(),
            pedersen_param: todo!(),
            am_werewolf: todo!(),
            game_state: todo!(),
            player_randomness: todo!(),
            player_commitment: todo!(),
        };

        todo!();

        // let proof_id = request_proof_with_output(
        //     CircuitIdentifier::Built(BuiltinCircuit::WinningJudge(winning_judge_circuit)),
        //     ProofOutputType::Public,
        // )
        // .await?;

        // if check_status_with_retry(&proof_id).await? {
        //     let result = if alive_werewolves == 0 {
        //         GameResult::VillagerWin
        //     } else if alive_werewolves >= alive_villagers {
        //         GameResult::WerewolfWin
        //     } else {
        //         GameResult::InProgress
        //     };

        //     drop(games);
        //     let mut games = state.games.lock().await;
        //     let game = games.get_mut(room_id).ok_or("Game not found")?;
        //     game.result = result.clone();
        //     Ok(result)
        // } else {
        //     Err("勝利判定の証明に失敗しました".to_string())
        // }
    } else {
        let result = if alive_werewolves == 0 {
            GameResult::VillagerWin
        } else if alive_werewolves >= alive_villagers {
            GameResult::WerewolfWin
        } else {
            GameResult::InProgress
        };

        drop(games);
        let mut games = state.games.lock().await;
        let game = games.get_mut(room_id).ok_or("Game not found")?;

        if result != GameResult::InProgress {
            let (winner_message, details) = match result {
                GameResult::VillagerWin => (
                    "Villagers team wins!",
                    format!("Remaining villagers: {}", alive_villagers),
                ),
                GameResult::WerewolfWin => (
                    "Werewolves team wins!",
                    format!(
                        "Remaining werewolves: {}, Remaining villagers: {}",
                        alive_werewolves, alive_villagers
                    ),
                ),
                GameResult::InProgress => unreachable!(),
            };

            game.chat_log.add_message(ChatMessage::new(
                "system".to_string(),
                "System".to_string(),
                format!("{} {}", winner_message, details),
                ChatMessageType::System,
            ));

            game.phase = GamePhase::Finished;
        }

        game.result = result.clone();
        Ok(result)
    }
}

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

// make and store zkp parameters
pub async fn preprocessing_werewolf(state: AppState, game: &mut Game) -> Result<(), String> {
    if !state.debug_config.create_crypto_parameters {
        if state.debug_config.create_proof {
            return Err("create_crypto_parameters is required".to_string());
        }
        return Ok(());
    }

    let num_players = game.players.len();

    println!("num_players: {}", num_players);

    let rng = &mut test_rng();

    // generate pedersen_commitment parameters
    // TODO: revise. generate randomness secretly
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng).unwrap();

    let player_randomness = (0..num_players).map(|_| Fr::rand(rng)).collect::<Vec<_>>();

    let player_commitment = player_randomness
        .clone()
        .iter()
        .map(|r| {
            <Fr as LocalOrMPC<Fr>>::PedersenComScheme::commit(
                &pedersen_param,
                &r.convert_input(),
                &<Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // hoge.
    let mut pub_key_or_dummy_x = vec![Fr::from(0); num_players];
    let mut pub_key_or_dummy_y = vec![Fr::from(0); num_players];
    let is_fortune_teller = vec![Fr::from(0); num_players];

    // generate elgamal parameters

    println!("generate elgamal parameters");
    let elgamal_param = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme::setup(rng).unwrap();

    // fortune teller public key
    let (pk, sk) =
        <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme::keygen(&elgamal_param, rng).unwrap();
    pub_key_or_dummy_x[1] = pk.x;
    pub_key_or_dummy_y[1] = pk.y;

    // let mpc_input = WerewolfKeyInput::rand(rng);
    // let key_publicize_circuit = KeyPublicizeCircuit { mpc_input };

    todo!();

    // let mpc_input = WerewolfKeyInput::init();
    // mpc_input.set_public_input(rng, None);

    // // TODO: revise. generate randomness secretly & fix error occured here.
    // mpc_input.set_private_input(Some((
    //     pub_key_or_dummy_x,
    //     pub_key_or_dummy_y,
    //     is_fortune_teller,
    // )));
    // mpc_input.generate_input(rng);

    // let key_publicize_circuit = KeyPublicizeCircuit {
    //     mpc_input: mpc_input.clone(),
    // };

    // println!("proof request");

    // // proof request
    // let proof_id = request_proof_with_output(
    //     CircuitIdentifier::Built(BuiltinCircuit::KeyPublicize(key_publicize_circuit)),
    //     ProofOutputType::Public,
    // )
    // .await?;

    // println!("proof id: {}", proof_id);

    // 各ゲームのZKPパラメータを更新
    let crypto_parameters = Some(crate::models::game::CryptoParameters {
        pedersen_param,
        player_randomness,
        player_commitment: player_commitment.clone(),
        fortune_teller_public_key: pk,
        elgamal_param,
        secret_key: sk,
    });

    game.crypto_parameters = crypto_parameters.clone();

    Ok(())
}

// 暗号パラメータの初期化関数
fn initialize_crypto_parameters(game: &mut Game) {
    let mut rng = test_rng();

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

    // プレイヤーごとのランダムネスとコミットメント（空で初期化、後でクライアントから受信）
    let player_randomness: Vec<Fr> = Vec::new();
    let player_commitment: Vec<
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::Output,
    > = Vec::new();

    game.crypto_parameters = Some(crate::models::game::CryptoParameters {
        pedersen_param,
        player_randomness,
        player_commitment,
        fortune_teller_public_key: pk,
        elgamal_param,
        secret_key: sk,
    });

    tracing::info!("Initialized crypto parameters for game {}", game.room_id);
}
