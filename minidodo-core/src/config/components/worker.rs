use crate::config::shared::PostgresConfig;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct WorkerServiceConfig {
    #[serde(default = "default_psp_base_url")]
    pub psp_base_url: String,
    #[serde(default = "default_sweep_interval_secs")]
    pub sweep_interval_secs: u64,
}

fn default_psp_base_url() -> String {
    "http://psp:3000".to_string()
}
fn default_sweep_interval_secs() -> u64 {
    30
}

impl Default for WorkerServiceConfig {
    fn default() -> Self {
        Self {
            psp_base_url: default_psp_base_url(),
            sweep_interval_secs: default_sweep_interval_secs(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    #[serde(default)]
    pub worker: WorkerServiceConfig,
    #[serde(default)]
    pub postgres: PostgresConfig,
}
