pub mod fs;
pub mod inode;
pub mod superblock;
pub mod file_handle;
pub mod block;
pub mod error;
mod args;

pub use crate::error::Result;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, Duration};
use clap::Parser;
use fuser::{FileAttr, FileType, Filesystem, KernelConfig, MountOption, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite, Request, TimeOrNow, FUSE_ROOT_ID};
use libc::{c_int, EIO, ENOENT};
use log::info;
use users::{get_current_gid, get_current_uid};


fn main() {
    env_logger::init();
}
