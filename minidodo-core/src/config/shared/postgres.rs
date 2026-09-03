use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PostgresConfig {
    #[serde(default = "default_db_host")]
    pub host: String,
    #[serde(default = "default_db_port")]
    pub port: u16,
    #[serde(default = "default_db_name")]
    pub database: String,
    #[serde(default = "default_db_user")]
    pub user: String,
    #[serde(default = "default_db_password")]
    pub password: String,
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

fn default_db_host() -> String {
    "localhost".to_string()
}
fn default_db_port() -> u16 {
    5432
}
fn default_db_name() -> String {
    "minidodo".to_string()
}
fn default_db_user() -> String {
    "postgres".to_string()
}
fn default_db_password() -> String {
    "postgres".to_string()
}
fn default_pool_size() -> u32 {
    10
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: default_db_host(),
            port: default_db_port(),
            database: default_db_name(),
            user: default_db_user(),
            password: default_db_password(),
            pool_size: default_pool_size(),
        }
    }
}

impl PostgresConfig {
    pub fn to_connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}
