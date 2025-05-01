use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::{Duration, SystemTime};
use fuser::FUSE_ROOT_ID;
use serde::{Deserialize, Serialize};
use crate::block::BlockRef;
use crate::fs::BLOCK_SIZE;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SuperBlock {
    magic: u64,
    block_size: u32, 
    inode_count: u64,
    next_inode_id: u64,
    next_block_id: u64,
    root_dir_inode: u64,
    create_at: u64,
    dirty: bool,
}

impl SuperBlock {
    pub fn new() -> Self {
        Self {
            // TimeFS in hex
            magic: 0x54_69_6d_65_46_53,
            // 4KB
            block_size: BLOCK_SIZE,
            inode_count: FUSE_ROOT_ID,
            next_inode_id: FUSE_ROOT_ID + 1,
            next_block_id: 1,
            root_dir_inode: FUSE_ROOT_ID,
            dirty: false,
            create_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs(),
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> crate::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(bincode::deserialize_from(reader)?)
    }
    
    pub fn get_next_inode_id(&mut self) -> u64 {
        let id = self.next_inode_id;
        self.next_inode_id += 1;
        id
    }
    
    pub fn get_next_block_id(&mut self) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }
    
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }
    
    pub fn new_block(&mut self) -> BlockRef {
        let id = self.get_next_block_id();
        BlockRef::new(id)
    }
    
    pub fn alloc_inode(&mut self) {
        self.inode_count += 1;
    }
}