use thiserror::Error;

pub type MeshResult<T> = std::result::Result<T, MeshError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MeshError {
    #[error("mesh is disabled")]
    Disabled,
    #[error("mesh packet ttl expired")]
    TtlExpired,
    #[error("mesh packet duplicate")]
    Duplicate,
    #[error("relay denied: {0}")]
    RelayDenied(String),
    #[error("no route to {0}")]
    NoRoute(String),
    #[error("mesh decode failed: {0}")]
    Decode(String),
}
