pub mod components;
pub mod loader;
pub mod shared;

pub use components::{ApiServerConfig, ServerConfig};
pub use shared::PostgresConfig;

const POSTGRES_SECTIONS: &[&str] = &["postgres"];
const SERVER_SECTIONS: &[&str] = &["server", "postgres"];

pub fn load_database_config() -> crate::Result<PostgresConfig> {
    loader::load_config_inner("postgres", "postgres", POSTGRES_SECTIONS)
}

pub fn load_server_config() -> crate::Result<ServerConfig> {
    loader::load_config_typed("server", SERVER_SECTIONS)
}
