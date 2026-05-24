/// 平台适配器 trait — 各平台（Windows WCF / Linux / Android）实现此接口
#[cfg(feature = "windows-cfapi")]
use async_trait::async_trait;
#[cfg(feature = "windows-cfapi")]
use crate::errors::Result;
#[cfg(feature = "windows-cfapi")]
use crate::models::{LocalFileEvent, RemoteFileEntry};
#[cfg(feature = "windows-cfapi")]
use std::path::Path;
#[cfg(feature = "windows-cfapi")]
use tokio::sync::mpsc;

#[cfg(feature = "windows-cfapi")]
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

#[cfg(feature = "windows-cfapi")]
pub mod wcf;
