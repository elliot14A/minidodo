use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MockPspConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_webhook_signing_secret")]
    pub webhook_signing_secret: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    3000
}
fn default_webhook_signing_secret() -> String {
    "whsec_test_secret_12345".to_string()
}

impl Default for MockPspConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            webhook_signing_secret: default_webhook_signing_secret(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct PspConfig {
    #[serde(default)]
    pub psp: MockPspConfig,
}
