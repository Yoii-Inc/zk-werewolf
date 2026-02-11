use dotenvy::dotenv;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_test_env() {
    INIT.call_once(|| {
        dotenv().ok();
        // バックアップ値を設定（.envファイルが存在しない場合のデフォルト値）
        if std::env::var("SUPABASE_URL").is_err() {
            std::env::set_var("SUPABASE_URL", "https://test-project.supabase.co");
        }
        if std::env::var("SUPABASE_KEY").is_err() {
            std::env::set_var("SUPABASE_KEY", "test-key");
        }
        if std::env::var("ZK_MPC_NODE_0_HTTP").is_err() {
            std::env::set_var("ZK_MPC_NODE_0_HTTP", "http://localhost:9000");
        }
        if std::env::var("ZK_MPC_NODE_1_HTTP").is_err() {
            std::env::set_var("ZK_MPC_NODE_1_HTTP", "http://localhost:9001");
        }
        if std::env::var("ZK_MPC_NODE_2_HTTP").is_err() {
            std::env::set_var("ZK_MPC_NODE_2_HTTP", "http://localhost:9002");
        }
        if std::env::var("JWT_SECRET").is_err() {
            std::env::set_var("JWT_SECRET", "test-jwt-secret");
        }
        if std::env::var("BLOCKCHAIN_ENABLED").is_err() {
            std::env::set_var("BLOCKCHAIN_ENABLED", "false");
        }
        if std::env::var("ETHEREUM_RPC_URL").is_err() {
            std::env::set_var("ETHEREUM_RPC_URL", "http://localhost:8545");
        }
        if std::env::var("ETHEREUM_CHAIN_ID").is_err() {
            std::env::set_var("ETHEREUM_CHAIN_ID", "31337");
        }
    });
}
