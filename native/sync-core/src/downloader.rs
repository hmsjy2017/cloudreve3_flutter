use crate::api_client::ApiClient;
use crate::errors::{Result, SyncError};
use crate::file_lock::FileLockRegistry;
use crate::models::*;
use crate::sync_db::SyncDb;
use tokio::sync::Semaphore;

/// 下载单个文件（含重试），受并发信号量控制
#[allow(clippy::too_many_arguments)]
pub async fn download_file(
    task_id: &str,
    action: &SyncAction,
    config: &WorkerConfig,
    api: &ApiClient,
    db: &SyncDb,
    file_locks: &FileLockRegistry,
    semaphore: &Semaphore,
    root_id: &str,
) -> Result<()> {
    let remote = action.remote_entry.as_ref().ok_or_else(|| {
        SyncError::Internal("下载操作缺少远程文件信息".into())
    })?;

    let _lock = file_locks.acquire(&action.relative_path).await;

    if remote.is_dir {
        let local_path = config.local_root.join(&action.relative_path);
        tokio::fs::create_dir_all(&local_path).await?;
        tracing::debug!("[{}] 创建本地目录: {}", task_id, action.relative_path);
        return Ok(());
    }

    let local_path = config.local_root.join(&action.relative_path);

    let _permit = semaphore.acquire().await
        .map_err(|e| SyncError::Internal(format!("获取传输信号量失败: {}", e)))?;

    tracing::info!("[{}] 开始下载: {} ({}bytes)", task_id, action.relative_path, remote.size);

    // 确保父目录存在
    if let Some(parent) = local_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let max_retries = 3u32;
    let mut attempt = 0u32;

    loop {
        attempt += 1;

        let urls = match api.get_download_url(&[&remote.uri]).await {
            Ok(urls) => urls,
            Err(SyncError::Auth(_)) => return Err(SyncError::Auth("Token 过期".into())),
            Err(e) if attempt <= max_retries => {
                let delay = crate::utils::retry_delay_ms(attempt, 1000, 30000);
                tracing::warn!("[{}] 下载重试 ({}/{}): 获取链接失败: {}", task_id, attempt, max_retries, e);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            Err(e) => return Err(e),
        };

        let download_url = match urls.first() {
            Some(u) => u.clone(),
            None => {
                tracing::error!("[{}] 未获取到下载链接, uri={}", task_id, remote.uri);
                return Err(SyncError::Network("未获取到下载链接".into()));
            }
        };

        let resp = match api.stream_download(&download_url, 0).await {
            Ok(r) => r,
            Err(SyncError::Auth(_)) => return Err(SyncError::Auth("Token 过期".into())),
            Err(e) if attempt <= max_retries => {
                let delay = crate::utils::retry_delay_ms(attempt, 1000, 30000);
                tracing::warn!("[{}] 下载重试 ({}/{}): 连接失败: {}", task_id, attempt, max_retries, e);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            Err(e) => return Err(e),
        };

        let tmp_path = local_path.with_extension(".sync_tmp");

        match stream_to_file(resp, &tmp_path, config.bandwidth_limit).await {
            Ok(_) => {
                tracing::debug!("[{}] 下载写入完成: {} ({}bytes)", task_id, tmp_path.display(), remote.size);
                tokio::fs::rename(&tmp_path, &local_path).await?;

                if remote.mtime_ms > 0 {
                    let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_millis(remote.mtime_ms as u64);
                    let _ = filetime::set_file_mtime(&local_path, filetime::FileTime::from_system_time(mtime));
                }

                let local_hash = crate::utils::quick_hash(&local_path, remote.size).await.unwrap_or_default();
                db.upsert_file_mapping(&FileMapping {
                    id: 0,
                    sync_root_id: root_id.to_string(),
                    local_path: std::path::PathBuf::from(&action.relative_path),
                    remote_uri: remote.uri.clone(),
                    remote_file_id: remote.file_id.clone(),
                    local_hash: Some(local_hash.clone()),
                    remote_hash: remote.hash.clone().or(Some(local_hash)),
                    local_mtime: Some(remote.mtime_ms),
                    remote_mtime: Some(remote.mtime_ms),
                    local_size: Some(remote.size),
                    remote_size: Some(remote.size),
                    sync_status: SyncFileStatus::Synced,
                    is_placeholder: false,
                }).await?;

                tracing::info!("[{}] 下载完成: {}", task_id, action.relative_path);
                return Ok(());
            }
            Err(e) if attempt <= max_retries => {
                let delay = crate::utils::retry_delay_ms(attempt, 1000, 30000);
                tracing::warn!("[{}] 下载重试 ({}/{}): 写入失败: {}", task_id, attempt, max_retries, e);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err(e);
            }
        }
    }
}

/// 流式写入文件（含带宽限速）
/// 限速策略：先尽快从 HTTP 流读取数据到内存缓冲（避免连接超时），
/// 再按限速节奏写入磁盘
pub async fn stream_to_file(
    resp: reqwest::Response,
    tmp_path: &std::path::Path,
    bandwidth_limit: Option<u64>,
) -> Result<()> {
    let mut file = tokio::fs::File::create(tmp_path).await?;
    use tokio::io::AsyncWriteExt;
    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;

    // 无限速：直接流式写入
    if bandwidth_limit.is_none() {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| SyncError::Network(e.to_string()))?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        return Ok(());
    }

    // 有限速：先快速读取到内存，再按节奏写磁盘
    let limit = bandwidth_limit.unwrap();
    let mut all_chunks: Vec<bytes::Bytes> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| SyncError::Network(e.to_string()))?;
        all_chunks.push(chunk);
    }

    let transfer_start = std::time::Instant::now();
    let mut total_bytes: u64 = 0;

    for chunk in &all_chunks {
        total_bytes += chunk.len() as u64;
        file.write_all(chunk).await?;

        let expected_elapsed = std::time::Duration::from_micros(
            total_bytes * 1_000_000 / limit
        );
        let actual_elapsed = transfer_start.elapsed();
        if expected_elapsed > actual_elapsed {
            tokio::time::sleep(expected_elapsed - actual_elapsed).await;
        }
    }
    file.flush().await?;
    Ok(())
}
