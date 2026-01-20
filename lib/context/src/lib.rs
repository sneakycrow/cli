use base64::{Engine, engine::general_purpose};
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub use crate::error::SneakyContextError;
mod error;

/// Context for all sneaky crow related functionality
#[derive(Serialize, Deserialize, Clone)]
pub struct SneakyContext {
    pub me: SneakyMeta,
    pub build_info: SneakyBuildInfo,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SneakyMeta {
    pub name: String,
    pub email: String,
    pub title: String,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct SneakyBuildInfo {
    pub hash: String,
}

impl Default for SneakyBuildInfo {
    fn default() -> Self {
        let date_time = Utc::now().to_string();
        let build_hash = format!("dev-{date_time}");
        let b64_build_hash = general_purpose::STANDARD.encode(build_hash);
        SneakyBuildInfo {
            hash: b64_build_hash,
        }
    }
}

pub const DEFAULT_CONFIG_FILE: &str = "sc.toml";
pub const DEFAULT_NAME: &str = "Zachary Corivdae";
pub const DEFAULT_EMAIL: &str = "zach@sneakycrow.dev";
pub const DEFAULT_TITLE: &str = "Artist & Engineer";

impl SneakyContext {
    /// Derives the context from a configuration file
    pub fn from_file(file: &str) -> Result<Self, SneakyContextError> {
        let config = std::fs::read_to_string(file)?;
        let meta: SneakyMeta = toml::from_str(&config)?;
        let build_info = {
            if let Ok(env_build_hash) = std::env::var("BUILD_HASH") {
                SneakyBuildInfo {
                    hash: env_build_hash,
                }
            } else {
                SneakyBuildInfo::default()
            }
        };
        Ok(Self {
            me: meta,
            build_info,
        })
    }
}

impl Default for SneakyContext {
    fn default() -> Self {
        Self {
            me: SneakyMeta {
                name: DEFAULT_NAME.to_string(),
                email: DEFAULT_EMAIL.to_string(),
                title: DEFAULT_TITLE.to_string(),
            },
            build_info: SneakyBuildInfo::default(),
        }
    }
}
