use std::time::SystemTime;
use fuser::{FileAttr, FileType};
use users::{get_current_gid, get_current_uid};
use crate::fs::BLOCK_SIZE;

pub(crate) struct FileAttrBuilder {
    ino: u64,
    size: u64,
    blocks: u64,
    atime: SystemTime,
    mtime: SystemTime,
    ctime: SystemTime,
    crtime: SystemTime,
    kind: FileType,
    perm: u16,
    nlink: u32,
    uid: u32,
    gid: u32,
    rdev: u32,
    flags: u32,
    blksize: u32,
}

impl FileAttrBuilder {
    pub fn ino(mut self, ino: u64) -> Self {
        self.ino = ino;
        self
    }
    
    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }
    
    pub fn blocks(mut self, blocks: u64) -> Self {
        self.blocks = blocks;
        self
    }
    
    pub fn atime(mut self, atime: SystemTime) -> Self {
        self.atime = atime;
        self
    }
    
    pub fn mtime(mut self, mtime: SystemTime) -> Self {
        self.mtime = mtime;
        self
    }
    
    pub fn ctime(mut self, ctime: SystemTime) -> Self {
        self.ctime = ctime;
        self
    }
    
    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        self.crtime = crtime;
        self
    }
    
    pub fn kind(mut self, kind: FileType) -> Self {
        self.kind = kind;
        self
    }
    
    pub fn perm(mut self, perm: u16) -> Self {
        self.perm = perm;
        self
    }
    
    pub fn nlink(mut self, nlink: u32) -> Self {
        self.nlink = nlink;
        self
    }
    
    pub fn uid(mut self, uid: u32) -> Self {
        self.uid = uid;
        self
    }
    
    pub fn gid(mut self, gid: u32) -> Self {
        self.gid = gid;
        self
    }
    
    pub fn rdev(mut self, rdev: u32) -> Self {
        self.rdev = rdev;
        self
    }
    
    pub fn blksize(mut self, blksize: u32) -> Self {
        self.blksize = blksize;
        self
    }
    
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self.blocks = (self.size + BLOCK_SIZE as u64) / BLOCK_SIZE as u64;
        self.blksize = BLOCK_SIZE;
        self
    }
    
    pub fn zero_size(mut self) -> Self {
        self.size = 0;
        self.blocks = 0;
        self.blksize = BLOCK_SIZE;
        self
    }
    
    pub fn with_now(mut self) -> Self {
        let now = SystemTime::now();
        self.atime = now;
        self.mtime = now;
        self.ctime = now;
        self.crtime = now;
        self
    }
    
    pub fn with_current_user(mut self) -> Self {
        self.uid = get_current_uid();
        self.gid = get_current_gid();
        self
    }
    
    pub fn with_regular_file(mut self) -> Self {
        self.kind = FileType::RegularFile;
        self
    }
    
    pub fn with_directory(mut self) -> Self {
        self.kind = FileType::Directory;
        self.nlink = 2;
        self
    }
    
    pub fn with_owner_read_write_other_read_write(mut self) -> Self {
        self.perm = 0o755;
        self
    }
    
    pub fn with_owner_read_write_other_read(mut self) -> Self {
        self.perm = 0o644;
        self
    }
    
    pub fn with_everyone_read_write(mut self) -> Self {
        self.perm = 0o777;
        self
    }
    
    pub fn build(self) -> FileAttr {
        FileAttr {
            ino: self.ino,
            size: self.ino,
            blocks: self.ino,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.crtime,
            kind: self.kind,
            perm: self.perm,
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: self.rdev,
            flags: self.flags,
            blksize: self.blksize,
        }
    }
}

impl Default for FileAttrBuilder {
    fn default() -> Self {
        let now = SystemTime::now();
        
        Self {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::RegularFile,
            perm: 0o755,
            nlink: 1,
            uid: get_current_uid(),
            gid: get_current_gid(),
            rdev: 0,
            flags: 0,
            blksize: BLOCK_SIZE,
        }
    }
}