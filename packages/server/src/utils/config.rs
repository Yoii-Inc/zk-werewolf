use once_cell::sync::Lazy;
use std::env;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::new());

pub struct Config {
    pub supabase_url: String,
    pub supabase_key: String,
    pub jwt_secret: String,
}

impl Config {
    fn new() -> Self {
        Self {
            supabase_url: env::var("SUPABASE_URL").expect("SUPABASE_URL must be set"),
            supabase_key: env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        }
    }
}
