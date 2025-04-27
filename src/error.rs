use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum TimeFSError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialize error: {0}")]
    Serialize(#[from] bincode::Error),
    #[error("Failed to find inode: {0}")]
    NotFound(u64),
    #[error("Inode {0} is not a folder")]
    NotDirectory(u64),
    #[error("block index error")]
    BlockIndexError,
}

pub type Result<T> = std::result::Result<T, TimeFSError>;