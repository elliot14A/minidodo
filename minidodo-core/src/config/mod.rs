pub mod components;
pub mod loader;
pub mod shared;

pub use components::{ApiServerConfig, MockPspConfig, PspConfig, ServerConfig, WorkerConfig, WorkerServiceConfig};
pub use shared::PostgresConfig;

const POSTGRES_SECTIONS: &[&str] = &["postgres"];
const SERVER_SECTIONS: &[&str] = &["server", "postgres"];
const PSP_SECTIONS: &[&str] = &["psp"];
const WORKER_SECTIONS: &[&str] = &["worker", "postgres"];

pub fn load_database_config() -> crate::Result<PostgresConfig> {
    loader::load_config_inner("postgres", "postgres", POSTGRES_SECTIONS)
}

pub fn load_server_config() -> crate::Result<ServerConfig> {
    loader::load_config_typed("server", SERVER_SECTIONS)
}

pub fn load_psp_config() -> crate::Result<PspConfig> {
    loader::load_config_typed("psp", PSP_SECTIONS)
}

pub fn load_worker_config() -> crate::Result<WorkerConfig> {
    loader::load_config_typed("worker", WORKER_SECTIONS)
}
