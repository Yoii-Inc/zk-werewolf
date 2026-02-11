use once_cell::sync::Lazy;
use std::env;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::new());

#[derive(Clone)]
pub struct Config {
    pub supabase_url: String,
    pub supabase_key: String,
    pub jwt_secret: String,
    pub zk_mpc_node_0: String,
    pub zk_mpc_node_1: String,
    pub zk_mpc_node_2: String,
    pub blockchain_enabled: bool,
    pub ethereum_rpc_url: String,
    pub ethereum_chain_id: u64,
    pub deployer_private_key: String,
    pub werewolf_game_contract: String,
    pub werewolf_verifier_contract: String,
    pub werewolf_rewards_contract: String,
}

impl Config {
    fn new() -> Self {
        Self {
            supabase_url: env::var("SUPABASE_URL").expect("SUPABASE_URL must be set"),
            supabase_key: env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            zk_mpc_node_0: env::var("ZK_MPC_NODE_0_HTTP").expect("ZK_MPC_NODE_0_HTTP must be set"),
            zk_mpc_node_1: env::var("ZK_MPC_NODE_1_HTTP").expect("ZK_MPC_NODE_1_HTTP must be set"),
            zk_mpc_node_2: env::var("ZK_MPC_NODE_2_HTTP").expect("ZK_MPC_NODE_2_HTTP must be set"),
            blockchain_enabled: env::var("BLOCKCHAIN_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_ascii_lowercase()
                == "true",
            ethereum_rpc_url: env::var("ETHEREUM_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8545".to_string()),
            ethereum_chain_id: env::var("ETHEREUM_CHAIN_ID")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(31337),
            deployer_private_key: env::var("DEPLOYER_PRIVATE_KEY").unwrap_or_default(),
            werewolf_game_contract: env::var("WEREWOLF_GAME_CONTRACT").unwrap_or_default(),
            werewolf_verifier_contract: env::var("WEREWOLF_VERIFIER_CONTRACT").unwrap_or_default(),
            werewolf_rewards_contract: env::var("WEREWOLF_REWARDS_CONTRACT").unwrap_or_default(),
        }
    }

    pub fn zk_mpc_node_urls(&self) -> Vec<String> {
        vec![
            self.zk_mpc_node_0.clone(),
            self.zk_mpc_node_1.clone(),
            self.zk_mpc_node_2.clone(),
        ]
    }
}
