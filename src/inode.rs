use crate::block::BlockRef;
use crate::{from_bin_file, write_to_bin_file, AutoSave, Result};
use fuser::FileAttr;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::path::Path;
use crate::error::TimeFSError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum INodeType {
    File {
        blocks: Vec<BlockRef>,
        size: u64,
    },
    Directory {
        entries: HashMap<String, u64>,
    }
}

impl INodeType {
    pub fn empty_file() -> Self {
        INodeType::File {
            blocks: Vec::new(),
            size: 0,
        }
    }
    
    pub fn empty_directory() -> Self {
        INodeType::Directory {
            entries: HashMap::new()
        }
    }
}
 #[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct INode {
    pub(crate) id: u64,
    pub(crate) parent: u64,
    pub(crate) data: INodeType,
    pub(crate) attr: FileAttr,
}

impl INode {
    pub fn new(
        id: u64,
        parent: u64,
        data: INodeType,
        attr: FileAttr,
    ) -> Self {
        Self { id, parent, data, attr }
    }
    
    pub fn with_file_size(id: u64, block_id: u64, parent: u64, attr: FileAttr, size: u64) -> Self {
        let data = INodeType::File { 
            blocks: BlockRef::alloc_blocks(block_id, size),
            size,
        };
        Self::new(id, parent, data, attr)
    }
    
    pub fn with_directory_entries(id: u64, parent: u64, attr: FileAttr, entries: HashMap<String, u64>) -> Self {
        let data = INodeType::Directory {
            entries,
        };
        Self::new(id, parent, data, attr)
    }
    
    pub fn new_autosave(
        id: u64,
        parent: u64,
        data: INodeType,
        attr: FileAttr,
        inode_dir: &Path,
    ) -> AutoSave<Self> {
        let val = Self::new(id, parent, data, attr);
        let path = inode_dir.join(format!("inode_{}.bin", id));
        AutoSave::new(val, path)
    }
    
    pub fn write_to_file(&self, inode_dir: &Path) -> Result<()> {
        let path = inode_dir.join(format!("inode_{}.bin", self.id));
        write_to_bin_file(self, path.as_path())?;
        Ok(())
    }
    
    pub fn from_file(id: u64, inode_dir: &Path) -> Result<Self> {
        let path = inode_dir.join(format!("inode_{}.bin", id));
        Ok(from_bin_file(path.as_path())?)
    }
    
    pub fn from_file_autosave(id: u64, inode_dir: &Path) -> Result<AutoSave<Self>> {
        let path = inode_dir.join(format!("inode_{}.bin", id));
        let val = Self::from_file(id, &path)?;
        Ok(AutoSave::new(val, path))
    }
    
    pub fn is_file(&self) -> bool {
        if let INodeType::File { .. } = self.data {
            true
        } else {
            false
        }
    }
    
    pub fn is_directory(&self) -> bool {
        if let INodeType::Directory { .. } = self.data {
            true
        } else {
            false
        }
    }
    
    pub fn get_child_id(&self, name: impl AsRef<str>) -> Result<u64> {
        let name = name.as_ref();
        
        match self.data {
            INodeType::File { .. } => Err(TimeFSError::NotDirectory(self.id)),
            INodeType::Directory {
                ref entries,
            } => entries.get(name).map(|e| *e).ok_or(TimeFSError::NameNotFound(name.to_string()))
        }
    }
}
