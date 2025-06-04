use once_cell::sync::Lazy;
use std::env;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::new());

#[derive(Clone)]
pub struct Config {
    pub supabase_url: String,
    pub supabase_key: String,
    pub jwt_secret: String,
    pub zk_mpc_node_1: String,
    pub zk_mpc_node_2: String,
    pub zk_mpc_node_3: String,
}

impl Config {
    fn new() -> Self {
        Self {
            supabase_url: env::var("SUPABASE_URL").expect("SUPABASE_URL must be set"),
            supabase_key: env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            zk_mpc_node_1: env::var("ZK_MPC_NODE_1").expect("ZK_MPC_NODE_1 must be set"),
            zk_mpc_node_2: env::var("ZK_MPC_NODE_2").expect("ZK_MPC_NODE_2 must be set"),
            zk_mpc_node_3: env::var("ZK_MPC_NODE_3").expect("ZK_MPC_NODE_3 must be set"),
        }
    }

    pub fn zk_mpc_node_urls(&self) -> Vec<&str> {
        vec![
            &self.zk_mpc_node_1,
            &self.zk_mpc_node_2,
            &self.zk_mpc_node_3,
        ]
    }
}
