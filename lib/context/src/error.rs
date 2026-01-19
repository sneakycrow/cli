use thiserror::Error;

#[derive(Error, Debug)]
pub enum SneakyContextError {
    #[error("TOML deserialization error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
}
