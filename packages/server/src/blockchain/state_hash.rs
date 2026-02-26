use crate::models::game::{Game, GamePhase, GameResult};
use serde_json::Value;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;

pub fn compute_game_id(room_id: &str) -> [u8; 32] {
    let mut input = Vec::with_capacity(8 + room_id.len());
    input.extend_from_slice(b"game_id:");
    input.extend_from_slice(room_id.as_bytes());
    hash_bytes(&input)
}

pub fn compute_proof_id(seed: &str) -> [u8; 32] {
    let mut input = Vec::with_capacity(9 + seed.len());
    input.extend_from_slice(b"proof_id:");
    input.extend_from_slice(seed.as_bytes());
    hash_bytes(&input)
}

pub fn compute_game_state_hash(game: &Game) -> [u8; 32] {
    let mut input = Vec::new();

    input.extend_from_slice(b"room:");
    input.extend_from_slice(game.room_id.as_bytes());

    input.extend_from_slice(b"|phase:");
    input.push(phase_to_u8(&game.phase));

    input.extend_from_slice(b"|day:");
    input.extend_from_slice(&game.day_count.to_le_bytes());

    input.extend_from_slice(b"|result:");
    input.push(result_to_u8(&game.result));

    // Players are sorted by ID for deterministic hashing.
    let mut players = game.players.clone();
    players.sort_by(|a, b| a.id.cmp(&b.id));
    for p in players {
        input.extend_from_slice(b"|player:");
        input.extend_from_slice(p.id.as_bytes());
        input.push(if p.is_dead { 1 } else { 0 });
        input.push(if p.is_ready { 1 } else { 0 });
    }

    // Include high-level action/vote state for reproducibility.
    input.extend_from_slice(b"|attacks:");
    input.extend_from_slice(&(game.night_actions.attacks.len() as u64).to_le_bytes());
    for attack in &game.night_actions.attacks {
        input.extend_from_slice(attack.as_bytes());
    }

    input.extend_from_slice(b"|votes:");
    input.extend_from_slice(&(game.vote_results.len() as u64).to_le_bytes());

    let mut vote_keys = game.vote_results.keys().cloned().collect::<Vec<_>>();
    vote_keys.sort();
    for key in vote_keys {
        if let Some(vote) = game.vote_results.get(&key) {
            input.extend_from_slice(key.as_bytes());
            input.extend_from_slice(&(vote.voters.len() as u64).to_le_bytes());
            let mut voters = vote.voters.clone();
            voters.sort();
            for voter in voters {
                input.extend_from_slice(voter.as_bytes());
            }
        }
    }

    hash_bytes(&input)
}

pub fn compute_commitment_hash(room_id: &str, player_id: &str, commitment: &Value) -> [u8; 32] {
    let mut input = Vec::new();
    input.extend_from_slice(room_id.as_bytes());
    input.extend_from_slice(player_id.as_bytes());
    input.extend_from_slice(commitment.to_string().as_bytes());
    hash_bytes(&input)
}

pub fn bytes32_to_hex(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(66);
    out.push_str("0x");
    for b in bytes {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0x0f) as usize]);
    }
    out
}

pub fn is_evm_address(value: &str) -> bool {
    if value.len() != 42 || !value.starts_with("0x") {
        return false;
    }

    value
        .as_bytes()
        .iter()
        .skip(2)
        .all(|b| b.is_ascii_hexdigit())
}

pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        let seed = FNV_OFFSET_BASIS ^ ((i as u64 + 1) * 0x9e3779b97f4a7c15);
        let part = fnv1a64_with_seed(bytes, seed).to_le_bytes();
        let start = i * 8;
        out[start..start + 8].copy_from_slice(&part);
    }
    out
}

fn fnv1a64_with_seed(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn phase_to_u8(phase: &GamePhase) -> u8 {
    match phase {
        GamePhase::Waiting => 0,
        GamePhase::Night => 1,
        GamePhase::DivinationProcessing => 2,
        GamePhase::Discussion => 3,
        GamePhase::Voting => 4,
        GamePhase::Result => 5,
        GamePhase::Finished => 6,
    }
}

fn result_to_u8(result: &GameResult) -> u8 {
    match result {
        GameResult::InProgress => 0,
        GameResult::VillagerWin => 1,
        GameResult::WerewolfWin => 2,
    }
}

const HEX: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];
