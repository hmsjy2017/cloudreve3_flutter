//! FUSE (Linux) 相关的 SyncEngine 方法
//! 仅在 linux-fuse feature 启用时编译

use crate::models::*;

use super::SyncEngine;

#[cfg(feature = "linux-fuse")]
impl SyncEngine {
    /// MirrorFUSE: 处理 FUSE read 水合请求（按需下载）
    pub(crate) async fn handle_fuse_read(
        &self,
        request: crate::platform::fuse::FuseFetchRequest,
        _local_root: &std::path::Path,
    ) {
        let remote_uri = &request.remote_uri;
        let offset = request.offset;
        let length = request.length;

        tracing::debug!("FUSE 水合请求: uri={}, offset={}, length={}", remote_uri, offset, length);

        let root_id = match &self.sync_root_id {
            Some(id) => id.clone(),
            None => {
                let _ = request.reply_tx.send(Err("sync_root_id 为空".to_string()));
                return;
            }
        };

        // 清理过期缓存（超过 5 分钟）
        let now = std::time::Instant::now();
        self.hydration_cache.retain(|_, (_, ts)| now.duration_since(*ts).as_secs() < 300);

        // 尝试从缓存获取已下载的数据
        let data = if let Some(cached) = self.hydration_cache.get(remote_uri) {
            tracing::debug!("FUSE 水合缓存命中: {}", remote_uri);
            cached.0.clone()
        } else {
            tracing::info!("FUSE 水合下载: {}", remote_uri);
            let config = self.snapshot_worker_config().await;

            let download_result = async {
                let urls = self.api.get_download_url(&[remote_uri]).await;
                let urls = match urls {
                    Ok(u) => u,
                    Err(crate::errors::SyncError::Auth(_)) => {
                        tracing::info!("FUSE 水合: token 过期，尝试刷新后重试");
                        self.api.refresh_access_token().await?;
                        self.api.get_download_url(&[remote_uri]).await?
                    }
                    Err(e) => return Err(e),
                };
                let download_url = urls.into_iter().next()
                    .ok_or_else(|| crate::errors::SyncError::Network("获取下载 URL 返回空列表".into()))?;

                let data = crate::downloader::download_to_buffer(
                    &self.api,
                    &download_url,
                    config.bandwidth_limit,
                ).await?;

                Ok::<Vec<u8>, crate::errors::SyncError>(data)
            }.await;

            match download_result {
                Ok(data) => {
                    self.hydration_cache.insert(remote_uri.clone(), (data.clone(), std::time::Instant::now()));
                    data
                }
                Err(e) => {
                    tracing::error!("FUSE 水合下载失败: {}: {}", remote_uri, e);
                    let _ = request.reply_tx.send(Err(format!("下载失败: {}", e)));
                    return;
                }
            }
        };

        // 更新 DB 映射（标记为已水合）
        if let Ok(Some(mapping)) = self.db.find_mapping_by_remote_uri(&root_id, remote_uri).await {
            let _ = self.db.upsert_file_mapping(&FileMapping {
                id: mapping.id,
                sync_root_id: mapping.sync_root_id,
                local_path: mapping.local_path.clone(),
                remote_uri: mapping.remote_uri.clone(),
                remote_file_id: mapping.remote_file_id.clone(),
                local_hash: None,
                remote_hash: mapping.remote_hash.clone(),
                local_mtime: mapping.local_mtime,
                remote_mtime: mapping.remote_mtime,
                local_size: None,
                remote_size: Some(data.len() as u64),
                sync_status: SyncFileStatus::Synced,
                is_placeholder: false,
            }).await;
        }

        let _ = request.reply_tx.send(Ok(data));
    }

    /// MirrorFUSE: 为远程文件注册 FUSE inode（持续同步时远程新建/修改文件调用）
    pub(crate) async fn _create_placeholder_for_remote(
        &self,
        relative: &str,
        remote: &RemoteFileEntry,
        _local_root: &std::path::Path,
        _root_id: &str,
    ) {
        let adapter = match self.fuse_adapter.lock() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!("FUSE adapter lock 失败: {}", e);
                return;
            }
        };
        if let Some(ref fuse) = *adapter {
            let parent_rel = std::path::PathBuf::from(relative)
                .parent()
                .map(|p| crate::utils::normalize_path(&p.to_string_lossy()))
                .unwrap_or_default();

            let name = std::path::PathBuf::from(relative)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            fuse.create_placeholder_for_remote(
                &parent_rel,
                &name,
                relative,
                remote.is_dir,
                remote.size,
                &remote.uri,
                remote.hash.as_deref(),
                remote.mtime_ms,
            );
        }
    }

    /// FUSE 清理（卸载挂载点）
    pub(crate) fn cleanup_fuse(&self) {
        let adapter_opt = match self.fuse_adapter.lock() {
            Ok(mut guard) => guard.take(),
            Err(e) => {
                tracing::error!("FUSE adapter lock 失败: {}", e);
                return;
            }
        };
        if let Some(adapter) = adapter_opt {
            if let Err(e) = adapter.unmount() {
                tracing::warn!("FUSE 卸载失败: {}", e);
            }
            tracing::info!("FUSE 适配器已清理");
        }
    }
}
