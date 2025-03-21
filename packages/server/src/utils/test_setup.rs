use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_test_env() {
    INIT.call_once(|| {
        std::env::set_var("SUPABASE_URL", "https://test-project.supabase.co");
        std::env::set_var("SUPABASE_KEY", "test-key");
        std::env::set_var("JWT_SECRET", "test-jwt-secret");
    });
}