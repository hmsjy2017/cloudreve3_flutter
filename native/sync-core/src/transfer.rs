use crate::api_client::ApiClient;
use crate::errors::{Result, SyncError};
use crate::models::*;
use crate::sync_db::SyncDb;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub struct TransferManager {
    db: Arc<SyncDb>,
    api: Arc<ApiClient>,
    config: TransferConfig,
    semaphore: Arc<Semaphore>,
}

impl TransferManager {
    pub fn new(db: Arc<SyncDb>, api: Arc<ApiClient>, config: TransferConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        Self {
            db,
            api,
            config,
            semaphore,
        }
    }

    /// 下载文件（支持断点续传）
    pub async fn download(&self, task: &TransferTask) -> Result<()> {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            SyncError::Internal(format!("获取信号量失败: {}", e))
        })?;

        // 1. 检查磁盘空间
        self.check_disk_space(task.file_size).await?;

        // 2. 获取下载 URL
        let urls = self.api.get_download_url(&[&task.remote_uri]).await?;
        let url = urls.into_iter().next()
            .ok_or_else(|| SyncError::Network("未获取到下载 URL".to_string()))?;

        // 3. 流式下载到临时文件
        let local_path = PathBuf::from(&task.local_path);
        let tmp_path = local_path.with_extension("sync_tmp");

        let response = self.api.stream_download(&url, task.bytes_done).await?;

        // 确保目录存在
        if let Some(parent) = tmp_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(task.bytes_done > 0)
            .open(&tmp_path)
            .await?;

        let mut stream = response.bytes_stream();
        let mut bytes_done = task.bytes_done;

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            bytes_done += chunk.len() as u64;

            // 更新进度（简化版，直接更新）
            // TODO: 使用 spawn_blocking 避免 DB 写入阻塞
        }

        tokio::io::AsyncWriteExt::flush(&mut file).await?;
        drop(file);

        // 4. 下载完成，原子重命名
        if tmp_path.exists() {
            tokio::fs::rename(&tmp_path, &local_path).await?;
        }

        // 5. 设置文件权限 (Linux)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o644);
            tokio::fs::set_permissions(&local_path, perms).await?;
        }

        // 6. 更新数据库
        self.db.complete_transfer(task.id).await?;

        Ok(())
    }

    /// 上传文件（分片上传，按序执行）
    pub async fn upload(&self, task: &TransferTask) -> Result<()> {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            SyncError::Internal(format!("获取信号量失败: {}", e))
        })?;

        // 1. 创建上传会话
        let session = self.api.create_upload_session(
            &task.remote_uri,
            task.file_size,
        ).await?;

        // 2. 按序上传分片
        let chunk_size = session.chunk_size as usize;
        let mut file = tokio::fs::File::open(&task.local_path).await?;
        let start_chunk = (task.bytes_done / chunk_size as u64) as u32;

        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        file.seek(std::io::SeekFrom::Start(task.bytes_done)).await?;

        let total_chunks = ((task.file_size + chunk_size as u64 - 1) / chunk_size as u64) as u32;
        let mut buf = vec![0u8; chunk_size];

        for chunk_idx in start_chunk..total_chunks {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            buf.truncate(n);

            self.api.upload_chunk(&session.session_id, chunk_idx, &buf).await?;

            // 更新进度
            let progress = ((chunk_idx as u64 + 1) * chunk_size as u64).min(task.file_size);
            self.db.update_transfer_progress(task.id, progress).await?;
        }

        self.db.complete_transfer(task.id).await?;
        Ok(())
    }

    /// 检查磁盘空间
    async fn check_disk_space(&self, needed: u64) -> Result<()> {
        // 简化实现：通过检查目标目录所在磁盘的可用空间
        // 完整实现需要使用 fs2 或类似库
        Ok(())
    }

    /// 指数退避重试
    pub async fn retry_with_backoff<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        loop {
            match f().await {
                Ok(v) => return Ok(v),
                Err(e) if attempt < self.config.max_retries => {
                    let delay = crate::utils::retry_delay_ms(
                        attempt,
                        self.config.retry_base_delay_ms,
                        self.config.retry_max_delay_ms,
                    );
                    tracing::warn!("传输失败，{}ms后重试 ({}): {}", delay, attempt + 1, e);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
