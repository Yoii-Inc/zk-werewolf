use crate::models::game::GameResult;
use crate::utils::config::Config;
use std::sync::Arc;

pub mod state_hash;

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
enum Backend {
    Disabled,
    Simulated(SimulatedBackend),
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
        }
    }

    pub async fn verify_proof(
        &self,
        proof_id: [u8; 32],
        proof_data: &[u8],
        public_inputs: &[u8],
    ) -> Result<Option<bool>, String> {
        match &*self.backend {
            Backend::Disabled => Ok(None),
            Backend::Simulated(_) => {
                let verified = true;
                tracing::info!(
                    "[simulated-chain] verify_proof proof_id={} verified={} proof_bytes={} public_inputs_bytes={}",
                    state_hash::bytes32_to_hex(&proof_id),
                    verified,
                    proof_data.len(),
                    public_inputs.len()
                );
                Ok(Some(verified))
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
        }
    }
}

fn simulated_tx_hash(label: &str, seed: [u8; 32]) -> String {
    let mut input = Vec::with_capacity(label.len() + seed.len());
    input.extend_from_slice(label.as_bytes());
    input.extend_from_slice(&seed);
    let hash = state_hash::hash_bytes(&input);
    state_hash::bytes32_to_hex(&hash)
}
