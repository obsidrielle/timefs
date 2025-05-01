use libc::c_int;
use thiserror::Error;
use crate::block::BlockCacheError;

#[derive(Debug, Error)]
pub enum TimeFSError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialize error: {0}")]
    Serialize(#[from] bincode::Error),
    #[error("Failed to find inode: {0}")]
    NotFound(u64),
    #[error("Failed to find by name {0}")]
    NameNotFound(String),
    #[error("Inode {0} is not a folder")]
    NotDirectory(u64),
    #[error("Inode {0} is a folder")]
    IsDirectory(u64),
    #[error("Name {0} has existed")]
    NameExist(String),
    #[error("block index error")]
    BlockIndexError,
    #[error("{0}")]
    BlockCacheError(#[from] BlockCacheError)
}

pub type Result<T> = std::result::Result<T, TimeFSError>;

impl Into<c_int> for TimeFSError {
    fn into(self) -> c_int {
        match self {
            Self::Io(_) => libc::EIO,
            Self::NotFound(_) => libc::ENOENT,
            Self::NameNotFound(_) => libc::ENOENT,
            Self::NotDirectory(_) => libc::ENOTDIR,
            Self::IsDirectory(_) => libc::EISDIR,
            Self::NameExist(_) => libc::EEXIST,
            Self::BlockIndexError => libc::EINVAL,
            Self::BlockCacheError(_) => libc::EIO,
            _ => libc::EIO,
        }
    }
}