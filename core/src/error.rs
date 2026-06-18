use thiserror::Error;

#[derive(Debug, Error)]
pub enum DIDError {
    #[error("Invalid DID: {0}")]
    InvalidDid(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Invalid multibase: {0}")]
    InvalidMultibase(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Orphaned topic {topic_id}: {reason}")]
    OrphanedTopic { topic_id: String, reason: String },
    #[error("Representation not supported: {0}")]
    RepresentationNotSupported(String),
}
