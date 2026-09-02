use crate::{MinidodoError, Result, SystemErrorCode};
use figment::{Figment, providers::Env};
use serde::Deserialize;
use std::collections::HashSet;

fn create_figment_for(allowed_top_level: &[&str]) -> Figment {
    let allowed: HashSet<String> = allowed_top_level
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();

    let allowed1 = allowed.clone();
    let allowed2 = allowed;

    Figment::new()
        .merge(
            Env::prefixed("MINIDODO_")
                .map(|k| {
                    if k.as_str() == "POSTGRES_POOL_SIZE" {
                        "POSTGRES.POOL_SIZE".into()
                    } else {
                        k.as_str().replace('_', ".").into()
                    }
                })
                .filter(move |k| {
                    let top = k
                        .as_str()
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    allowed1.contains(&top)
                }),
        )
        .merge(
            Env::raw()
                .map(|k| {
                    if k.as_str() == "POSTGRES_POOL_SIZE" {
                        "POSTGRES.POOL_SIZE".into()
                    } else {
                        k.as_str().replace('_', ".").into()
                    }
                })
                .filter(move |k| {
                    let top = k
                        .as_str()
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    allowed2.contains(&top)
                }),
        )
}

pub fn load_config_typed<T>(config_name: &str, allowed_top_level: &[&str]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    create_figment_for(allowed_top_level)
        .extract()
        .map_err(|e| {
            tracing::error!(config_name = %config_name, error = %e, "Failed to load configuration");
            MinidodoError::Internal {
                message: format!("Failed to load {} configuration: {}", config_name, e),
                code: SystemErrorCode::CONFIG_ERROR,
            }
        })
}

pub fn load_config_inner<T>(config_name: &str, key: &str, allowed_top_level: &[&str]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    create_figment_for(allowed_top_level)
        .extract_inner(key)
        .map_err(|e| {
            tracing::error!(config_name = %config_name, error = %e, "Failed to load configuration");
            MinidodoError::Internal {
                message: format!("Failed to load {} configuration: {}", config_name, e),
                code: SystemErrorCode::CONFIG_ERROR,
            }
        })
}
