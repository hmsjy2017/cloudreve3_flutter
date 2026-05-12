mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
pub mod api;
pub mod models;
pub mod errors;
pub mod utils;
pub mod sync_db;
pub mod api_client;
pub mod fs_scanner;
pub mod conflict_resolver;
pub mod transfer;
pub mod event_handler;
pub mod sync_engine;

// 平台适配器 trait
use async_trait::async_trait;
use crate::errors::Result;
use crate::models::{LocalFileEvent, RemoteFileEntry};
use std::path::Path;
use tokio::sync::mpsc;

#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    /// 初始同步后的平台初始化
    async fn post_initial_sync(&self, config: &crate::models::SyncConfig) -> Result<()>;

    /// 监听本地文件变化，返回事件接收器
    fn watch_local_changes(&self) -> Result<mpsc::Receiver<LocalFileEvent>>;

    /// 创建本地文件占位符（Windows）或实际文件（Linux）
    async fn create_local_entry(&self, entry: &RemoteFileEntry, local_path: &Path) -> Result<()>;

    /// 水合文件
    async fn hydrate_file(&self, local_path: &Path, remote_url: &str) -> Result<()>;

    /// 脱水文件
    async fn dehydrate_file(&self, local_path: &Path) -> Result<()>;

    /// 关闭平台监听
    async fn shutdown(&self) -> Result<()>;
}
