use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub mock_psp_url: String,
    pub port: u16,
    pub psp_timeout_secs: u64,
    pub webhook_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://dodo_user:dodo_password@localhost:5432/dodo_payments".to_string());
        let mock_psp_url = env::var("MOCK_PSP_URL").unwrap_or_else(|_| "http://localhost:9090".to_string());
        let port = env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080);
        let psp_timeout_secs = env::var("PSP_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        let webhook_timeout_secs = env::var("WEBHOOK_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            database_url,
            mock_psp_url,
            port,
            psp_timeout_secs,
            webhook_timeout_secs,
        }
    }
}
