use crate::models::game::GameResult;
use crate::utils::config::Config;
use std::sync::Arc;
use tokio::process::Command;

pub mod state_hash;

#[derive(Clone, Copy, Debug)]
pub enum ProofType {
    RoleAssignment,
    Divination,
    AnonymousVoting,
    WinningJudgement,
    KeyPublicize,
}

const ROLE_ASSIGNMENT_VERIFY_PROOF_GAS_LIMIT: u64 = 1_500_000;

impl ProofType {
    fn as_u8(self) -> u8 {
        match self {
            ProofType::RoleAssignment => 0,
            ProofType::Divination => 1,
            ProofType::AnonymousVoting => 2,
            ProofType::WinningJudgement => 3,
            ProofType::KeyPublicize => 4,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ContractAddresses {
    pub game_contract: String,
    pub verifier_contract: String,
    pub rewards_contract: String,
}

impl ContractAddresses {
    pub fn is_configured(&self) -> bool {
        !self.game_contract.is_empty()
            && !self.verifier_contract.is_empty()
            && !self.rewards_contract.is_empty()
    }
}

#[derive(Clone, Debug)]
struct SimulatedBackend {
    pub rpc_url: String,
    pub chain_id: u64,
    pub addresses: ContractAddresses,
}

#[derive(Clone, Debug)]
struct RealBackend {
    pub rpc_url: String,
    pub chain_id: u64,
    pub private_key: String,
    pub from_address: String,
    pub addresses: ContractAddresses,
}

impl RealBackend {
    fn from_config(config: &Config, addresses: &ContractAddresses) -> Result<Self, String> {
        let private_key = config.deployer_private_key.trim().to_string();
        if private_key.is_empty() {
            return Err("DEPLOYER_PRIVATE_KEY is empty".to_string());
        }
        if !command_available("cast") {
            return Err("cast command is not available in PATH".to_string());
        }

        if !state_hash::is_evm_address(&addresses.game_contract)
            || !state_hash::is_evm_address(&addresses.verifier_contract)
            || !state_hash::is_evm_address(&addresses.rewards_contract)
        {
            return Err("one or more contract addresses are invalid".to_string());
        }

        let normalized_private_key = normalize_private_key(&private_key);
        let from_address = derive_address_from_private_key(&normalized_private_key)?;

        Ok(Self {
            rpc_url: config.ethereum_rpc_url.clone(),
            chain_id: config.ethereum_chain_id,
            private_key: normalized_private_key,
            from_address,
            addresses: addresses.clone(),
        })
    }
}

#[derive(Clone, Debug)]
enum Backend {
    Disabled,
    Simulated(SimulatedBackend),
    Real(RealBackend),
}

#[derive(Clone, Debug)]
pub struct BlockchainClient {
    backend: Arc<Backend>,
}

impl BlockchainClient {
    pub fn new(config: &Config) -> Self {
        if !config.blockchain_enabled {
            return Self {
                backend: Arc::new(Backend::Disabled),
            };
        }

        let addresses = ContractAddresses {
            game_contract: config.werewolf_game_contract.clone(),
            verifier_contract: config.werewolf_verifier_contract.clone(),
            rewards_contract: config.werewolf_rewards_contract.clone(),
        };

        if !addresses.is_configured() {
            tracing::warn!(
                "BLOCKCHAIN_ENABLED=true but one or more contract addresses are empty. Falling back to disabled mode."
            );
            return Self {
                backend: Arc::new(Backend::Disabled),
            };
        }

        match RealBackend::from_config(config, &addresses) {
            Ok(backend) => {
                tracing::info!(
                    "Blockchain client initialized in real mode (rpc={}, chain_id={}, game_contract={}).",
                    backend.rpc_url,
                    backend.chain_id,
                    backend.addresses.game_contract
                );
                Self {
                    backend: Arc::new(Backend::Real(backend)),
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to initialize real blockchain backend ({}). Falling back to simulated mode.",
                    e
                );

                let backend = SimulatedBackend {
                    rpc_url: config.ethereum_rpc_url.clone(),
                    chain_id: config.ethereum_chain_id,
                    addresses,
                };
                tracing::info!(
                    "Blockchain client initialized in simulated mode (rpc={}, chain_id={}).",
                    backend.rpc_url,
                    backend.chain_id
                );

                Self {
                    backend: Arc::new(Backend::Simulated(backend)),
                }
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(&*self.backend, Backend::Disabled)
    }

    pub async fn create_game(
        &self,
        game_id: [u8; 32],
        players: Vec<String>,
    ) -> Result<Option<String>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(backend) => {
                let tx = simulated_tx_hash("create_game", game_id);
                tracing::info!(
                    "[simulated-chain] create_game game_id={} players={} tx={} game_contract={}",
                    state_hash::bytes32_to_hex(&game_id),
                    players.len(),
                    tx,
                    backend.addresses.game_contract
                );
                Ok(Some(tx))
            }
            Backend::Real(backend) => {
                let players = normalize_addresses("players", players)?;
                let players_arg = format!("[{}]", players.join(","));
                let mut args = base_send_args(
                    backend,
                    &backend.addresses.game_contract,
                    "createGame(bytes32,address[])",
                );
                args.push(state_hash::bytes32_to_hex(&game_id));
                args.push(players_arg);
                let tx = send_and_extract_tx_hash(&args).await?;
                tracing::info!(
                    "[real-chain] create_game game_id={} players={} tx={} game_contract={}",
                    state_hash::bytes32_to_hex(&game_id),
                    players.len(),
                    tx,
                    backend.addresses.game_contract
                );
                Ok(Some(tx))
            }
        }
    }

    pub async fn submit_commitment(
        &self,
        game_id: [u8; 32],
        player: &str,
        commitment: [u8; 32],
    ) -> Result<Option<String>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(_) => {
                let tx = simulated_tx_hash("submit_commitment", commitment);
                tracing::info!(
                    "[simulated-chain] submit_commitment game_id={} player={} commitment={} tx={}",
                    state_hash::bytes32_to_hex(&game_id),
                    player,
                    state_hash::bytes32_to_hex(&commitment),
                    tx
                );
                Ok(Some(tx))
            }
            Backend::Real(backend) => {
                let mut args = base_send_args(
                    backend,
                    &backend.addresses.game_contract,
                    "submitCommitment(bytes32,bytes32)",
                );
                args.push(state_hash::bytes32_to_hex(&game_id));
                args.push(state_hash::bytes32_to_hex(&commitment));
                let tx = send_and_extract_tx_hash(&args).await?;
                tracing::info!(
                    "[real-chain] submit_commitment game_id={} player={} commitment={} tx={}",
                    state_hash::bytes32_to_hex(&game_id),
                    player,
                    state_hash::bytes32_to_hex(&commitment),
                    tx
                );
                Ok(Some(tx))
            }
        }
    }

    pub async fn update_game_state(
        &self,
        game_id: [u8; 32],
        state_hash: [u8; 32],
    ) -> Result<Option<String>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(_) => {
                let tx = simulated_tx_hash("update_game_state", state_hash);
                tracing::info!(
                    "[simulated-chain] update_game_state game_id={} state_hash={} tx={}",
                    state_hash::bytes32_to_hex(&game_id),
                    state_hash::bytes32_to_hex(&state_hash),
                    tx
                );
                Ok(Some(tx))
            }
            Backend::Real(backend) => {
                let mut args = base_send_args(
                    backend,
                    &backend.addresses.game_contract,
                    "updateGameState(bytes32,bytes32)",
                );
                args.push(state_hash::bytes32_to_hex(&game_id));
                args.push(state_hash::bytes32_to_hex(&state_hash));
                let tx = send_and_extract_tx_hash(&args).await?;
                tracing::info!(
                    "[real-chain] update_game_state game_id={} state_hash={} tx={}",
                    state_hash::bytes32_to_hex(&game_id),
                    state_hash::bytes32_to_hex(&state_hash),
                    tx
                );
                Ok(Some(tx))
            }
        }
    }

    pub async fn finalize_game(
        &self,
        game_id: [u8; 32],
        result: GameResult,
    ) -> Result<Option<String>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(_) => {
                let payload = state_hash::hash_bytes(format!("{:?}", result).as_bytes());
                let tx = simulated_tx_hash("finalize_game", payload);
                tracing::info!(
                    "[simulated-chain] finalize_game game_id={} result={:?} tx={}",
                    state_hash::bytes32_to_hex(&game_id),
                    result,
                    tx
                );
                Ok(Some(tx))
            }
            Backend::Real(backend) => {
                let mut args = base_send_args(
                    backend,
                    &backend.addresses.game_contract,
                    "finalizeGame(bytes32,uint8)",
                );
                args.push(state_hash::bytes32_to_hex(&game_id));
                args.push(game_result_to_u8(&result).to_string());
                let tx = send_and_extract_tx_hash(&args).await?;
                tracing::info!(
                    "[real-chain] finalize_game game_id={} result={:?} tx={}",
                    state_hash::bytes32_to_hex(&game_id),
                    result,
                    tx
                );
                Ok(Some(tx))
            }
        }
    }

    pub async fn verify_proof(
        &self,
        proof_id: [u8; 32],
        game_id: [u8; 32],
        proof_type: ProofType,
        proof_data: &[u8],
        public_inputs: &[u8],
    ) -> Result<Option<bool>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(_) => {
                let verified = true;
                tracing::info!(
                    "[simulated-chain] verify_proof proof_id={} game_id={} proof_type={:?} verified={} proof_bytes={} public_inputs_bytes={}",
                    state_hash::bytes32_to_hex(&proof_id),
                    state_hash::bytes32_to_hex(&game_id),
                    proof_type,
                    verified,
                    proof_data.len(),
                    public_inputs.len()
                );
                Ok(Some(verified))
            }
            Backend::Real(backend) => {
                let proof_hex = bytes_to_hex(proof_data);
                let public_inputs_hex = bytes_to_hex(public_inputs);
                let proof_type_u8 = proof_type.as_u8();
                let verify_gas_limit = if matches!(proof_type, ProofType::RoleAssignment) {
                    Some(ROLE_ASSIGNMENT_VERIFY_PROOF_GAS_LIMIT)
                } else {
                    None
                };

                let call_args = vec![
                    "call".to_string(),
                    backend.addresses.verifier_contract.clone(),
                    "verifyProof(bytes32,bytes32,uint8,bytes,bytes)(bool)".to_string(),
                    state_hash::bytes32_to_hex(&proof_id),
                    state_hash::bytes32_to_hex(&game_id),
                    proof_type_u8.to_string(),
                    proof_hex.clone(),
                    public_inputs_hex.clone(),
                    "--from".to_string(),
                    backend.from_address.clone(),
                    "--rpc-url".to_string(),
                    backend.rpc_url.clone(),
                ];
                let mut call_args = call_args;
                if let Some(gas_limit) = verify_gas_limit {
                    call_args.push("--gas-limit".to_string());
                    call_args.push(gas_limit.to_string());
                }
                let call_output = run_cast(&call_args).await?;
                let preview_verified = parse_cast_bool(&call_output)?;
                if !preview_verified {
                    tracing::warn!(
                        "[real-chain] verify_proof static call returned false proof_id={} game_id={} proof_type={:?}",
                        state_hash::bytes32_to_hex(&proof_id),
                        state_hash::bytes32_to_hex(&game_id),
                        proof_type
                    );
                    return Ok(Some(false));
                }

                let mut send_args = base_send_args(
                    backend,
                    &backend.addresses.verifier_contract,
                    "verifyProof(bytes32,bytes32,uint8,bytes,bytes)",
                );
                send_args.push(state_hash::bytes32_to_hex(&proof_id));
                send_args.push(state_hash::bytes32_to_hex(&game_id));
                send_args.push(proof_type_u8.to_string());
                send_args.push(proof_hex);
                send_args.push(public_inputs_hex);
                if let Some(gas_limit) = verify_gas_limit {
                    send_args.push("--gas-limit".to_string());
                    send_args.push(gas_limit.to_string());
                }
                let tx = send_and_extract_tx_hash(&send_args).await?;

                tracing::info!(
                    "[real-chain] verify_proof proof_id={} game_id={} proof_type={:?} verified=true tx={}",
                    state_hash::bytes32_to_hex(&proof_id),
                    state_hash::bytes32_to_hex(&game_id),
                    proof_type,
                    tx
                );
                Ok(Some(true))
            }
        }
    }

    pub async fn distribute_rewards(
        &self,
        game_id: [u8; 32],
        winners: Vec<String>,
    ) -> Result<Option<String>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(backend) => {
                let tx = simulated_tx_hash("distribute_rewards", game_id);
                tracing::info!(
                    "[simulated-chain] distribute_rewards game_id={} winners={} tx={} rewards_contract={}",
                    state_hash::bytes32_to_hex(&game_id),
                    winners.len(),
                    tx,
                    backend.addresses.rewards_contract
                );
                Ok(Some(tx))
            }
            Backend::Real(backend) => {
                let winners = normalize_addresses("winners", winners)?;
                let winners_arg = format!("[{}]", winners.join(","));
                let mut args = base_send_args(
                    backend,
                    &backend.addresses.rewards_contract,
                    "distributeRewards(bytes32,address[])",
                );
                args.push(state_hash::bytes32_to_hex(&game_id));
                args.push(winners_arg);
                let tx = send_and_extract_tx_hash(&args).await?;
                tracing::info!(
                    "[real-chain] distribute_rewards game_id={} winners={} tx={} rewards_contract={}",
                    state_hash::bytes32_to_hex(&game_id),
                    winners.len(),
                    tx,
                    backend.addresses.rewards_contract
                );
                Ok(Some(tx))
            }
        }
    }
}

fn base_send_args(backend: &RealBackend, to: &str, sig: &str) -> Vec<String> {
    vec![
        "send".to_string(),
        to.to_string(),
        sig.to_string(),
        "--rpc-url".to_string(),
        backend.rpc_url.clone(),
        "--private-key".to_string(),
        backend.private_key.clone(),
        "--json".to_string(),
    ]
}

fn normalize_private_key(value: &str) -> String {
    let key = value.trim();
    if key.starts_with("0x") {
        key.to_string()
    } else {
        format!("0x{}", key)
    }
}

fn derive_address_from_private_key(private_key: &str) -> Result<String, String> {
    let output = std::process::Command::new("cast")
        .args(["wallet", "address", "--private-key", private_key])
        .output()
        .map_err(|e| format!("failed to execute cast wallet address: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cast wallet address failed (status={}): {}",
            output.status,
            stderr.trim()
        ));
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !state_hash::is_evm_address(&value) {
        return Err(format!(
            "cast wallet address returned invalid address: {}",
            value
        ));
    }
    Ok(value)
}

fn normalize_addresses(label: &str, values: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        if !state_hash::is_evm_address(&value) {
            return Err(format!("invalid {} address: {}", label, value));
        }
        out.push(value);
    }
    Ok(out)
}

fn command_available(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn send_and_extract_tx_hash(args: &[String]) -> Result<String, String> {
    let output = run_cast(args).await?;
    parse_tx_hash(&output)
}

async fn run_cast(args: &[String]) -> Result<String, String> {
    let output = Command::new("cast")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("failed to execute cast: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "cast command failed (status={}): stderr='{}' stdout='{}'",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_cast_bool(output: &str) -> Result<bool, String> {
    let value = output.trim().to_ascii_lowercase();
    if value == "true" {
        return Ok(true);
    }
    if value == "false" {
        return Ok(false);
    }
    if value.starts_with("0x") {
        if value.chars().skip(2).all(|c| c == '0') {
            return Ok(false);
        }
        return Ok(true);
    }
    Err(format!("unable to parse cast call bool output: {}", output))
}

fn parse_tx_hash(output: &str) -> Result<String, String> {
    let trimmed = output.trim();
    if trimmed.starts_with('{') {
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| format!("failed to parse cast json output: {}", e))?;
        if let Some(tx_hash) = value
            .get("transactionHash")
            .and_then(|v| v.as_str())
            .or_else(|| value.get("hash").and_then(|v| v.as_str()))
            .or_else(|| value.get("txHash").and_then(|v| v.as_str()))
        {
            return Ok(tx_hash.to_string());
        }
    }

    if trimmed.starts_with("0x") && trimmed.len() == 66 {
        return Ok(trimmed.to_string());
    }

    if let Some(token) = trimmed
        .split_whitespace()
        .find(|t| t.starts_with("0x") && t.len() == 66)
    {
        return Ok(token.to_string());
    }

    Err(format!(
        "failed to extract tx hash from cast output: {}",
        output
    ))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(2 + bytes.len() * 2);
    out.push_str("0x");
    for b in bytes {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0x0f) as usize]);
    }
    out
}

fn game_result_to_u8(result: &GameResult) -> u8 {
    match result {
        GameResult::InProgress => 0,
        GameResult::VillagerWin => 1,
        GameResult::WerewolfWin => 2,
    }
}

fn simulated_tx_hash(label: &str, seed: [u8; 32]) -> String {
    let mut input = Vec::with_capacity(label.len() + seed.len());
    input.extend_from_slice(label.as_bytes());
    input.extend_from_slice(&seed);
    let hash = state_hash::hash_bytes(&input);
    state_hash::bytes32_to_hex(&hash)
}

const HEX: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];
