pub mod fs;
pub mod inode;
pub mod superblock;
pub mod file_handle;
pub mod block;
pub mod error;
mod args;
mod file_attr;

use std::io::{BufReader, BufWriter};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde::de::DeserializeOwned;
use serde::Serialize;
pub use crate::error::Result;

pub(crate) fn from_bin_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    Ok(bincode::deserialize_from(reader)?)
}

pub(crate) fn write_to_bin_file<T: Serialize>(val: &T, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, val)?;
    Ok(())
}

pub(crate) fn from_bin_compressed_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let reader = ZlibDecoder::new(reader);
    Ok(bincode::deserialize_from(reader)?)
}

pub(crate) fn write_to_bin_compressed_file<T: Serialize>(val: &T, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);
    let writer = ZlibEncoder::new(writer, Compression::best());
    bincode::serialize_into(writer, val)?;
    Ok(())
}

pub(crate) struct AutoSave<T>
where T: Serialize {
    inner: T,
    path: PathBuf,
}

impl<'a, T> AutoSave<T> 
where T: Serialize {
    fn new(inner: T, path: impl AsRef<Path>) -> Self {
        Self { inner, path: path.as_ref().to_path_buf() }
    }
}

impl<T> Drop for AutoSave<T>
where T: Serialize {
    fn drop(&mut self) {
        write_to_bin_file(&self.inner, &self.path).expect("Failed to write to file");
    }
}

impl<T> Deref for AutoSave<T>
where T: Serialize {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for AutoSave<T>
where T: Serialize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

fn main() {
    env_logger::init();
}
