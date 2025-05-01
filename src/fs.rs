use std::collections::HashMap;
use std::ffi::OsStr;
use std::num::{NonZero, NonZeroUsize};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use dashmap::DashMap;
use fuser::{FileAttr, FileType, Filesystem, KernelConfig, ReplyAttr, ReplyCreate, Request, FUSE_ROOT_ID};
use libc::{c_int, EEXIST, EISDIR, ENOENT};
use log::{debug, error};
use parking_lot::{Mutex, RwLock};
use users::{get_current_gid, get_current_uid};
use crate::block::{BlockCache};
use crate::file_handle::FileHandle;
use crate::inode::{INode, INodeType};
use crate::superblock::SuperBlock;
use crate::{AutoSave, Result};
use crate::error::TimeFSError;
use crate::file_attr::FileAttrBuilder;

pub(crate) const BLOCK_SIZE: u32 = 4096;

pub(crate) struct TimeFS {
    mount_path: PathBuf,
    storage_path: PathBuf,
    metadata_dir: PathBuf,
    inode_dir: PathBuf,
    blocks_dir: PathBuf,
    super_block: RwLock<SuperBlock>,
    inodes: DashMap<u64, INode>,
    file_handles: DashMap<u64, FileHandle>,
    next_fs: Mutex<u64>,
    block_cache: Arc<BlockCache>,
} 

impl TimeFS {
    fn new(mount_path: impl AsRef<Path>, storage_path: impl AsRef<Path>) -> Result<Self> {
        let storage_path = storage_path.as_ref().to_path_buf();
        
        let metadata_dir = storage_path.join("metadata");
        let blocks_dir = storage_path.join("blocks");
        let inode_dir = metadata_dir.join("inode");

        std::fs::create_dir_all(&metadata_dir)?;
        std::fs::create_dir_all(&blocks_dir)?;
        std::fs::create_dir_all(&inode_dir)?;

        let super_block_path = metadata_dir.join("superblock.bin");
        let super_block = if super_block_path.exists() {
            SuperBlock::from_file(&super_block_path)?
        } else {
            let sb = SuperBlock::new();
            sb.write_to_file(&super_block_path)?;
            sb
        };
        
        let root_inode = Self::create_root_inode();
        root_inode.write_to_file(inode_dir.as_path())?;

        let mut inodes = DashMap::new();
        inodes.insert(FUSE_ROOT_ID, root_inode);

        let blocks_dir_cloned = blocks_dir.clone();
        
        Ok(Self {
            mount_path: mount_path.as_ref().to_path_buf(),
            storage_path,
            metadata_dir,
            blocks_dir,
            inode_dir,
            super_block: RwLock::new(super_block),
            inodes,
            file_handles: DashMap::new(),
            next_fs: Mutex::new(1),
            block_cache: Arc::new(BlockCache::new(1000, &blocks_dir_cloned, 30)),
        })
    }
    
    fn create_root_inode() -> INode {
        let uid = get_current_uid();
        let gid = get_current_gid();
        let now = SystemTime::now();
        
        let attr = FileAttr {
            ino: FUSE_ROOT_ID,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid,
            gid,
            rdev: 0,
            flags: 0,
            blksize: 4096,
        };
        
        INode::new(
            FUSE_ROOT_ID,
            FUSE_ROOT_ID,
            INodeType::empty_directory(),
            attr,
        )
    }
    
    fn get_next_inode_id(&self) -> u64 {
        let mut lock = self.super_block.write();
        lock.get_next_inode_id()
    }

    fn get_next_block_id(&self) -> u64 {
        let mut lock = self.super_block.write();
        lock.get_next_block_id()
    }

    fn get_inode(&self, id: u64) -> Result<impl Deref<Target = INode> + '_> {
        self.inodes
            .get(&id)
            .ok_or(TimeFSError::NotFound(id))
    }

    fn get_inode_mut(&self, id: u64) -> Result<impl DerefMut<Target = INode> + '_> {
        self.inodes
            .get_mut(&id)
            .ok_or(TimeFSError::NotFound(id))
    }

    fn get_inode_by_name(&self, parent: u64, name: impl AsRef<str>) -> Result<impl Deref<Target = INode> + '_> {
        let parent_node = self.get_inode(parent)?;
        let parent_node = parent_node.deref();
        let child_node = parent_node.get_child_id(name.as_ref())?;
        Ok(self.get_inode(child_node)?)
    }

    fn get_inode_mut_by_name(&self, parent: u64, name: impl AsRef<str>) -> Result<impl DerefMut<Target = INode> + '_> {
        let parent_node = self.get_inode(parent)?;
        let parent_node = parent_node.deref();
        let child_node = parent_node.get_child_id(name.as_ref())?;
        Ok(self.get_inode_mut(child_node)?)
    }

    fn create_file(&self, parent: u64, name: impl AsRef<str>, flags: i32) -> Result<(FileAttr, u64)> {
        let mut parent_node = self.get_inode_mut(parent)?;
        let parent_node = parent_node.deref_mut();

        let child_id = parent_node.get_child_id(name.as_ref())?;

        let inode = self.get_inode(child_id);

        if let Ok(inode) = inode {
            let inode = inode.deref();
            return Ok((inode.attr, self.alloc_file_handle(child_id, flags)));
        }

        let inode = self.alloc_inode(parent, FileType::RegularFile);
        let inode_id = inode.id;
        let attr = inode.attr;

        self.inodes.insert(child_id, inode);
        Ok((attr, inode_id))
    }

    fn alloc_inode(&self, parent: u64, kind: FileType) -> INode {
        let mut sb_lock = self.super_block.write();
        sb_lock.alloc_inode();

        let next_inode_id = sb_lock.get_next_inode_id();

        match kind {
            FileType::RegularFile =>  {
                let attr = FileAttrBuilder::default()
                    .ino(next_inode_id)
                    .build();

                INode::new(next_inode_id, parent, INodeType::empty_file(), attr)
            }
            FileType::Directory => {
                let attr = FileAttrBuilder::default()
                    .ino(next_inode_id)
                    .with_directory()
                    .build();

                INode::new(next_inode_id, parent, INodeType::empty_directory(), attr)
            }
            _ => unreachable!(),
        }
    }

    fn alloc_file_handle(&self, inode_id: u64, flags: i32) -> u64 {
        let mut lock = self.next_fs.lock();
        let handle_id = *lock;
        *lock += 1;

        self.file_handles.insert(handle_id, FileHandle::new(inode_id, flags));
        handle_id
    }

    fn get_attr(&self, inode_id: u64) -> Result<FileAttr> {
        let inode = self.get_inode(inode_id)?;
        let inode = inode.deref();
        Ok(inode.attr)
    }
}

impl Filesystem for TimeFS {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> std::result::Result<(), c_int> {
        debug!("TimeFS has inited");
        Ok(())
    }

    fn destroy(&mut self) {
        debug!("TimeFS has destroyed");
    }

    fn create(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, mode: u32, umask: u32, flags: i32, reply: ReplyCreate) {
        debug!("create(parent = {}, name = {:?}, mode = {}, umask = {}, flags = {})", parent, name, mode, umask, flags);

        let name_str= name.to_str();

        if name_str.is_none() {
            error!("{:?} is not a valid UTF-8 string", name_str);
            reply.error(libc::EINVAL);
            return;
        }

        let name_str = name_str.unwrap();

        match self.create_file(parent, name_str, flags) {
            Ok((attr, handle_id)) => {
                let ttl = std::time::Duration::from_secs(1);
                reply.created(&ttl, &attr, 0, handle_id, flags as u32);
            }
            Err(e) => reply.error(e.into())
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, fh: Option<u64>, reply: ReplyAttr) {

    }
}