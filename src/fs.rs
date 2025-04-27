use std::collections::HashMap;
use std::path::{Path, PathBuf};
use lru::LruCache;
use parking_lot::{Mutex, RwLock};
use crate::block::CacheEntry;
use crate::file_handle::FileHandle;
use crate::inode::INode;
use crate::superblock::SuperBlock;
use crate::Result;

pub(crate) struct TimeFS {
    mount_path: PathBuf,
    storage_path: PathBuf,
    metadata_path: PathBuf,
    blocks_dir: PathBuf,
    super_block: RwLock<SuperBlock>,
    inodes: RwLock<HashMap<u64, INode>>,
    file_handles: RwLock<HashMap<u64, FileHandle>>,
    next_fs: Mutex<u64>,
    block_cache: Mutex<LruCache<u64, CacheEntry>>,
} 

impl TimeFS {
    pub fn new(mount_path: impl AsRef<Path>, storage_path: impl AsRef<Path>) -> Result<Self> {
        let mut storage_path = storage_path.as_ref().to_path_buf();
        
        let metadata_dir = storage_path.join("metadata");
        let blocks_dir = storage_path.join("blocks");
        
        std::fs::create_dir_all(&metadata_dir)?;
        std::fs::create_dir_all(&blocks_dir)?;
        
        
    }
}