use serde::{Deserialize, Serialize};

pub use crate::error::SneakyContextError;
mod error;

/// Context for all sneaky crow related functionality
#[derive(Serialize, Deserialize, Clone)]
pub struct SneakyContext {
    pub me: SneakyMeta,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SneakyMeta {
    pub name: String,
    pub email: String,
    pub title: String,
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
        Ok(Self { me: meta })
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
        }
    }
}
