use std::path::Path;
use std::time::{Duration, SystemTime};
use fuser::FUSE_ROOT_ID;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SuperBlock {
    magic: u64,
    block_size: u32,
    inode_count: u64,
    next_inode_id: u64,
    root_dir_inode: u64,
    create_at: u64,
}

impl SuperBlock {
    pub fn new() -> Self {
        Self {
            // TimeFS in hex
            magic: 0x54_69_6d_65_46_53,
            // 4KB
            block_size: 4096,
            inode_count: FUSE_ROOT_ID,
            next_inode_id: FUSE_ROOT_ID + 1,
            root_dir_inode: FUSE_ROOT_ID,
            create_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs(),
        }
    }
    
    pub fn from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        
    }
}