use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BlockRef {
    block_id: u64,
    size: u32,
}

pub(crate) struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}