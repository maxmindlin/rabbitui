#[derive(Debug, Clone)]
pub struct AppConfig {
    pub update_rate: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            update_rate: 2_000,
        }
    }
}
