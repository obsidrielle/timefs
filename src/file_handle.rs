pub(crate) struct FileHandle {
    inode_id: u64,
    flags: i32, 
}

impl FileHandle {
    #[inline]
    pub(crate) fn new(inode_id: u64, flags: i32) -> Self {
        Self { inode_id, flags }
    }
} 


impl FileFlags for FileHandle {
    #[inline]
    fn is_read_only(&self) -> bool {
        self.flags.is_read_only()
    }

    #[inline]
    fn is_write_only(&self) -> bool {
        self.flags.is_write_only()
    }

    #[inline]
    fn is_read_write(&self) -> bool {
        self.flags.is_read_write()
    }

    #[inline]
    fn is_create(&self) -> bool {
        self.flags.is_create()
    }

    #[inline]
    fn is_truncate(&self) -> bool {
        self.flags.is_truncate()
    }

    #[inline]
    fn is_append(&self) -> bool {
        self.flags.is_append()
    }

    #[inline]
    fn is_sync(&self) -> bool {
        self.flags.is_sync()
    }
}

pub(crate) trait FileFlags {
    fn is_read_only(&self) -> bool;
    fn is_write_only(&self) -> bool;
    fn is_read_write(&self) -> bool;
    fn is_create(&self) -> bool;
    fn is_truncate(&self) -> bool;
    fn is_append(&self) -> bool;
    fn is_sync(&self) -> bool;
}

impl FileFlags for i32 {
    #[inline]
    fn is_read_only(&self) -> bool {
        self & libc::O_RDONLY != 0
    }

    #[inline]
    fn is_write_only(&self) -> bool {
        self & libc::O_WRONLY != 0
    }
    
    #[inline]
    fn is_read_write(&self) -> bool {
        self & libc::O_RDWR != 0
    }

    #[inline]
    fn is_create(&self) -> bool {
        self & libc::O_CREAT != 0
    }

    #[inline]
    fn is_truncate(&self) -> bool {
        self & libc::O_TRUNC != 0
    }

    #[inline]
    fn is_append(&self) -> bool {
        self & libc::O_APPEND != 0
    }

    #[inline]
    fn is_sync(&self) -> bool {
        self & libc::O_SYNC != 0
    }
}