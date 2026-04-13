use thiserror::Error;

#[derive(Error, Debug)]
pub enum CashewError {
    #[error("node not available — resolve the header first")]
    NodeNotAvailable,

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("CID creation failed: {0}")]
    CidCreationFailed(String),

    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("key not found for hash: {0}")]
    KeyNotFound(String),

    #[error("invalid IV data")]
    InvalidIV,

    #[error("decode from data error: {0}")]
    DecodeError(String),

    #[error("resolution error: {0}")]
    ResolutionError(String),

    #[error("transform failed: {0}")]
    TransformFailed(String),

    #[error("invalid key: {0}")]
    InvalidKey(String),

    #[error("missing data: {0}")]
    MissingData(String),

    #[error("proof error: {0}")]
    ProofError(String),

    #[error("invalid proof type: {0}")]
    InvalidProofType(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("invalid value: {0}")]
    InvalidValue(String),

    #[error("empty expression")]
    EmptyExpression,

    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
}

pub type Result<T> = std::result::Result<T, CashewError>;
