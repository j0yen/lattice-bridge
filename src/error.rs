//! Error type for lattice-bridge.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("OWL parse error: {0}")]
    OwlParse(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("{0}")]
    Other(String),
}
