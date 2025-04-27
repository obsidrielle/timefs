use crate::Result;
use std::collections::HashMap;
use std::fs::File;
use std::future;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam::channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use moka::future::{Cache, FutureExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::runtime;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use crate::error::TimeFSError;

#[derive(Error, Debug)]
pub enum BlockCacheError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to find block: {0}")]
    NotFound(u64),
    #[error("Failed to flush block: {0}")]
    FlushFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BlockRef {
    block_id: u64,
    size: u32,
}

#[derive(Clone)]
pub(crate) struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
    last_modified: Instant,
}

enum BlockOperation {
    MarkDirty(u64, Instant),
    Flush(u64),
    ShutDown,
}


type Blocks = Arc<Cache<u64, CacheEntry>>;
type DirtyTracer = Arc<DashMap<u64, Instant>>;
type BGHandle = Arc<Mutex<Option<std::thread::JoinHandle<()>>>>;

pub(crate) struct BlockCache {
    blocks: Blocks,
    dirty_tracer: DirtyTracer,
    operation_sender: Sender<BlockOperation>,
    blocks_dir: PathBuf,
    runtime: tokio::runtime::Handle,
    bg_handle: BGHandle,
}

impl BlockCache {
    pub fn new(max_capacity: u64, blocks_dir: &Path, flush_interval_secs: u64) -> Self {
        std::fs::create_dir_all(blocks_dir).expect("Failed to create block dir");

        let blocks_dir = blocks_dir.to_path_buf();
        let blocks_dir_cloned = blocks_dir.clone();
        let flush_blocks_dir = blocks_dir.to_path_buf();

        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .async_eviction_listener(move |key: Arc<u64>, entry: CacheEntry, _cause| {
                let blocks_dir_cloned = blocks_dir.clone();
                async move {
                    let path = Self::get_block_path_static(&blocks_dir_cloned, *key);
                    Self::write_block_to_disk(&path, &entry.data).await.expect("Failed to write block to disk");
                }.boxed()
            })
            .build();

        let (operation_sender, operation_receiver) = unbounded::<BlockOperation>();
        let runtime = tokio::runtime::Handle::current();

        let cache = Arc::new(cache);
        let flush_blocks = cache.clone();
        let dirty_tracer = Arc::new(DashMap::new());

        let dirty_tracer_cloned = dirty_tracer.clone();

        let handle = std::thread::spawn(move || {
            Self::background_thread(
                flush_blocks,
                flush_blocks_dir,
                dirty_tracer_cloned,
                operation_receiver,
                flush_interval_secs,
            )
        });

        Self {
            blocks: cache,
            dirty_tracer,
            operation_sender,
            blocks_dir: blocks_dir_cloned,
            runtime,
            bg_handle: Arc::new(Mutex::new(Some(handle))),
        }
    }

    fn get_block_path(&self, block_id: u64) -> PathBuf {
        let dir_id = block_id / 1000;
        let dir_path = self.blocks_dir.join(format!("{:03}", dir_id));

        let _ = std::fs::create_dir_all(&dir_path);
        dir_path.join(format!("block_{}.bin", block_id))
    }

    async fn get_block(&self, block_id: u64) -> Result<Vec<u8>> {
        if let Some(entry) = self.blocks.get(&block_id).await {
            return Ok(entry.data.clone());
        }

        let path = self.get_block_path(block_id);
        match tokio::fs::read(&path).await {
            Ok(data) => {
                self.blocks.insert(block_id, CacheEntry {
                    data: data.clone(),
                    dirty: false,
                    last_modified: Instant::now(),
                }).await;
                Ok(data)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    let empty_data = Vec::new();
                    self.blocks.insert(block_id, CacheEntry {
                        data: empty_data,
                        dirty: false,
                        last_modified: Instant::now(),
                    }).await;
                    Ok(vec![])
                } else { Err(BlockCacheError::Io(e).into()) }
            }
        }
    }

    async fn update_block(&self, block_id: u64, data: Vec<u8>) -> Result<()> {
        let now = Instant::now();

        self.blocks.insert(block_id, CacheEntry {
            data,
            dirty: true,
            last_modified: now,
        }).await;

        self.operation_sender.send(BlockOperation::MarkDirty(block_id, now))
            .map_err(|e| BlockCacheError::FlushFailed(e.to_string()))?;

        Ok(())
    }

    fn background_thread(
        blocks: Blocks,
        blocks_dir: PathBuf,
        dirty_tracer: DirtyTracer,
        operation_receiver: Receiver<BlockOperation>,
        flush_interval_secs: u64,
    ) {
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(12)
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime");

        runtime.block_on(async move {
            let dirty_cloned = dirty_tracer.clone();
            let blocks_dir_cloned = blocks_dir.clone();
            let blocks_cloned = blocks.clone();

            tokio::spawn(async move {
                Self::periodic_flush_task(
                    blocks_cloned,
                    blocks_dir_cloned,
                    dirty_cloned,
                    flush_interval_secs,
                ).await;
            });

            while let Ok(operation) = operation_receiver.recv() {
                match operation {
                    BlockOperation::MarkDirty(block_id, last_modified) => {
                        let dirty = dirty_tracer.clone();
                        dirty.insert(block_id, last_modified);
                    }
                    BlockOperation::Flush(block_id) => {
                        Self::flush_block_static(
                            block_id,
                            &blocks_dir,
                            blocks.clone(),
                            dirty_tracer.clone(),
                            false
                        ).await.expect("Failed to flush block");
                    }
                    BlockOperation::ShutDown => {
                        let dirty_block_ids = dirty_tracer
                                .iter()
                                .map(|e| *e.key())
                                .collect::<Vec<_>>();

                        for block_id in dirty_block_ids {
                            Self::flush_block_static(
                                block_id,
                                &blocks_dir,
                                blocks.clone(),
                                dirty_tracer.clone(),
                                true
                            ).await.expect("Failed to shut down cache!");
                        }

                        break;
                    }
                }
            }
        })
    }

    async fn flush_block(
        &self,
        block_id: u64,
        wait: bool,
    ) -> Result<bool> {
        Self::flush_block_static(
            block_id,
            &self.blocks_dir,
            self.blocks.clone(),
            self.dirty_tracer.clone(),
            wait,
        ).await
    }

    /// Flush a single block to disk.
    async fn flush_block_static(
        block_id: u64,
        blocks_dir: &Path,
        blocks: Blocks,
        dirty_blocks: DirtyTracer,
        wait: bool,
    ) -> Result<bool> {
        match blocks.get(&block_id).await {
            Some(entry) => {
                if entry.dirty {
                    let path = Self::get_block_path_static(blocks_dir, block_id);
                    let handle: JoinHandle<Result<()>> = tokio::spawn(async move {
                        if let Err(e) = Self::write_block_to_disk(&path, &entry.data).await {
                            return Err(e.into());
                        }
                        if let Some(mut entry) = blocks.get(&block_id).await {
                            entry.dirty = false;
                            blocks.insert(block_id, entry).await;
                        }

                        dirty_blocks.remove(&block_id);
                        Ok(())
                    });

                    if wait {
                        match handle.await {
                            Ok(_) => {},
                            Err(e) => {
                                return Err(BlockCacheError::FlushFailed("Failed to flush block!".into()).into());
                            }
                        }
                    }

                    Ok(true)
                } else { Ok(false) }
            }
            None => {
                Ok(false)
            }
        }
    }

    async fn periodic_flush_task(
        blocks: Blocks,
        blocks_dir: PathBuf,
        dirty_tracer: DirtyTracer,
        flush_interval_secs: u64,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let now = Instant::now();
            let flush_threshold = now - Duration::from_secs(flush_interval_secs);

            let blocks_to_flush = {
                let expired = dirty_tracer.iter()
                    .filter(|pair| *pair.value() <= flush_threshold)
                    .map(|pair| *pair.key())
                    .collect::<Vec<_>>();

                for id in &expired {
                    dirty_tracer.remove(id);
                }

                expired
            };

            for block_id in blocks_to_flush {
                if let Some(entry) = blocks.get(&block_id).await {
                    if entry.dirty {
                        let path = Self::get_block_path_static(&blocks_dir, block_id);
                        let data = entry.data.clone();

                        let blocks_ref = blocks.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::write_block_to_disk(&path, &data).await {
                                panic!("Failed to write block to disk: {:?}", e);
                            }
                            if let Some(mut entry) = blocks_ref.get(&block_id).await {
                                entry.dirty = false;
                                blocks_ref.insert(block_id, entry).await;
                            }
                        });
                    }
                }
            }
        }


    }

    async fn write_block_to_disk(path: &Path, data: &[u8]) -> Result<()> {
        let tmp_path = path.with_extension("tmp");
        let mut file = tokio::fs::File::create(&tmp_path).await?;

        file.write_all(data).await?;
        file.flush().await?;
        file.sync_all().await?;

        drop(file);

        tokio::fs::rename(tmp_path, path).await?;
        Ok(())
    }

    fn get_block_path_static(blocks_dir: &Path, block_id: u64) -> PathBuf {
        let dir_id = block_id / 1000;
        let dir_path = blocks_dir.join(format!("{:03}", dir_id));

        let _ = std::fs::create_dir_all(&dir_path);

        dir_path.join(format!("block_{}.bin", block_id))
    }

    async fn shutdown(&self) -> Result<()> {
        self.operation_sender.send(BlockOperation::ShutDown)
            .map_err(|e| BlockCacheError::FlushFailed(e.to_string()))?;

        let mut handle_lock = self.bg_handle.lock().await;

        if let Some(handle) = (*handle_lock).take() {
            if let Err(e) = handle.join() {
                return Err(BlockCacheError::FlushFailed(format!("Failed to join background thread: {:?}", e)).into());
            }
        }

        Ok(())
    }
}

impl BlockRef {
    pub fn new(id: u64) -> Self {
        Self {
            block_id: id,
            size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use libc::tm;
    use tempfile::{tempdir, TempDir};
    use super::*;

    fn setup_test_dir() -> TempDir {
        tempdir().expect("Failed to create test dir")
    }

    #[tokio::test]
    async fn test_basic_read_write() {
        let temp_dir = setup_test_dir();
        let cache_dir = temp_dir.path().to_path_buf();

        let mut cache = BlockCache::new(
            1000,
            &cache_dir,
            30,
        );

        let block_id = 42;
        let test_data = b"Hello, BlockCache!".to_vec();

        cache.update_block(block_id, test_data.clone()).await.unwrap();

        let read_data = cache.get_block(block_id).await.unwrap();
        assert_eq!(test_data, read_data);

        cache.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_persistence_after_shutdown() -> Result<()> {
        let temp_dir = setup_test_dir();
        let cache_dir = temp_dir.path().to_path_buf();

        let block_id = 300;
        let test_data = b"Persistent data".to_vec();

        {
            let mut cache = BlockCache::new(1000, &cache_dir, 30);
            cache.update_block(block_id, test_data.clone()).await?;
            cache.shutdown().await?;
        }
        {
            let mut cache = BlockCache::new(1000, &cache_dir, 30);
            let data = cache.get_block(block_id).await?;
            assert_eq!(test_data, data, "Should equal");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_delayed_flush() -> Result<()> {
        let temp_dir = setup_test_dir();
        let cache_dir = temp_dir.path().to_path_buf();

        let flush_interval_secs = 2;
        let mut cache = BlockCache::new(1000, &cache_dir, flush_interval_secs);

        let block_id = 200;
        let test_data = b"This will be auto-flushed".to_vec();

        cache.update_block(block_id, test_data.clone()).await?;

        let read_data = cache.get_block(block_id).await?;
        assert_eq!(test_data, read_data, "Data should be contained in mem");

        let block_path = cache_dir.join("000").join(format!("block_{}.bin", block_id));
        assert!(!block_path.exists() || std::fs::read(&block_path)? != test_data, "block shouldn't be flushed immediately");

        tokio::time::sleep(Duration::from_secs(flush_interval_secs + 5)).await;

        assert!(block_path.exists(), "block should be flushed to disk");

        let disk_data = tokio::fs::read(&block_path).await?;

        assert_eq!(test_data, disk_data);
        cache.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_flush_on_shutdown() -> Result<()> {
        let tempfile = setup_test_dir();
        let cache_dir = tempfile.path().to_path_buf();

        // a long interval
        let mut cache = BlockCache::new(1000, &cache_dir, 3600);

        let block_id = 42;
        let data = b"This will be flushed on shutdown".to_vec();

        cache.update_block(block_id, data.clone()).await?;

        let block_path = cache_dir.join("000").join(format!("block_{}.bin", block_id));
        assert!(!block_path.exists() || std::fs::read(&block_path)? != data, "block shouldn't be flushed immediately");

        cache.shutdown().await?;

        assert!(block_path.exists(), "block should be flushed to disk");
        let disk_data = tokio::fs::read(&block_path).await?;
        assert_eq!(data, disk_data);
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_access() -> Result<()> {
        let temp_dir = setup_test_dir();
        let cache_dir = temp_dir.path().to_path_buf();
        let mut handles = vec![];

        let cache = Arc::new(BlockCache::new(1000, &cache_dir, 30));

        for id in 0..100 {
            let cache_cloned = cache.clone();
            let block_id = id + 2000;
            let data = format!("Concurrent data {}", id).into_bytes();

            let handle = tokio::spawn(async move {
                cache_cloned.update_block(block_id, data.clone()).await?;

                let read_data = cache_cloned.get_block(block_id).await?;
                assert_eq!(data, read_data, "Concurrent data should be matched");

                tokio::time::sleep(Duration::from_millis(1000)).await;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(block_id)
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await.expect("Failed to join");
        }

        cache.shutdown().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_lru_eviction() -> Result<()> {
        let temp_dir = setup_test_dir();
        let cache_dir = temp_dir.path().to_path_buf();

        let max_blocks = 5;
        let cache = BlockCache::new(max_blocks, &cache_dir, 30);

        let block_count = max_blocks * 2;
        let mut block_data = HashMap::new();

        for i in 0..block_count {
            let block_id = i + 3000;
            let data = format!("LRU test data {}", i).into_bytes();

            cache.update_block(block_id, data.clone()).await?;
            block_data.insert(block_id, data);
            cache.flush_block(block_id, false).await?;
        }

        for (block_id, expected_data) in &block_data {
            let read_data = cache.get_block(*block_id).await?;
            assert_eq!(expected_data, &read_data, "Data should be equal");
        }

        cache.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_dirty_block_tracking() -> Result<()> {
        let temp_dir = setup_test_dir();
        let cache_dir = temp_dir.path().to_path_buf();

        let cache = BlockCache::new(1000, &cache_dir, 30);

        let block_id = 4000;
        let initial_data = b"Initial data".to_vec();
        let update_data = b"Update data".to_vec();

        cache.update_block(block_id, initial_data.clone()).await?;
        cache.flush_block(block_id, true).await?;

        let block_path = cache_dir.join("004").join(format!("block_{}.bin", block_id));
        let disk_data = std::fs::read(&block_path)?;
        assert_eq!(disk_data, initial_data, "Data should be equal");

        cache.update_block(block_id, update_data.clone()).await?;

        let disk_data = std::fs::read(&block_path)?;
        assert_eq!(disk_data, initial_data);

        let cached_data = cache.get_block(block_id).await?;
        assert_eq!(update_data, cached_data);

        cache.shutdown().await?;

        let final_data = std::fs::read(&block_path)?;
        assert_eq!(final_data, update_data);
        Ok(())
    }
}