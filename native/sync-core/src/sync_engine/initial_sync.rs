use crate::errors::Result;
use crate::fs_scanner::FsScanner;
use crate::models::*;
use std::time::Instant;

use super::SyncEngine;

impl SyncEngine {
    /// 初始全量同步
    pub async fn run_initial_sync(&self) -> Result<SyncSummary> {
        let start = Instant::now();

        // 重置 shutdown token，确保可重新启动
        let new_token = tokio_util::sync::CancellationToken::new();
        *self.shutdown_token.lock().unwrap() = new_token.clone();
        self.worker_pool.update_shutdown_token(new_token);

        *self.state.write().await = SyncState::Initializing;

        let (local_root, remote_root, sync_mode) = {
            let config = self.config.read().await;
            (config.local_root.clone(), config.remote_root.clone(), config.sync_mode.clone())
        };
        tracing::info!("开始初始同步, 模式={:?}", sync_mode);

        let scanner = FsScanner::new();
        tracing::info!("开始扫描本地文件系统: {}", local_root.display());
        let local_files = scanner.scan(&local_root, 50, false).await?;
        tracing::info!("本地扫描完成: {} 个条目", local_files.len());

        tracing::info!("开始扫描远程文件树: {}", remote_root);
        let remote_files = self.api.list_all_files(&remote_root).await?;
        tracing::info!("远程扫描完成: {} 个条目", remote_files.len());

        let db_mappings = self.load_all_mappings().await?;
        let plan = crate::diff::compute_diff(&local_files, &remote_files, &db_mappings, &remote_root, &sync_mode);
        tracing::info!(
            "差异计算完成: 上传={}, 下载={}, 删本地={}, 删远程={}, 冲突={}",
            plan.uploads.len(),
            plan.downloads.len(),
            plan.delete_local.len(),
            plan.delete_remote.len(),
            plan.conflicts.len(),
        );

        *self.state.write().await = SyncState::InitialSync {
            progress: InitialSyncProgress {
                scanned_local: local_files.len() as u64,
                scanned_remote: remote_files.len() as u64,
                total_to_sync: plan.total_actions(),
                ..Default::default()
            },
        };

        let worker_config = self.snapshot_worker_config().await;
        let conflict_resolver = self.conflict.read().await.clone();

        // MirrorWcf 模式：初始化 WCF 平台适配器
        #[cfg(feature = "windows-cfapi")]
        if matches!(sync_mode, SyncMode::MirrorWcf) {
            let config = self.config.read().await;
            let adapter = crate::platform::wcf::WcfPlatformAdapter::new(
                self.db.clone(),
                self.api.clone(),
                config.clone(),
            ).map_err(|e| crate::errors::SyncError::Internal(e.to_string()))?;
            let fetch_rx = adapter.take_fetch_receiver();
            *self.wcf_fetch_rx.lock().unwrap() = fetch_rx;
            let adapter_arc = std::sync::Arc::new(adapter);
            *self.platform_adapter.lock().unwrap() = Some(adapter_arc.clone());
            self.worker_pool.set_platform_adapter(adapter_arc);
            *self.cached_local_root.lock().unwrap() = config.local_root.clone();
            tracing::info!("MirrorWcf: WCF 平台适配器已初始化");
        }

        let summary = self.worker_pool.submit(
            plan, worker_config, WorkerTrigger::InitialSync, conflict_resolver,
        ).await?;

        *self.state.write().await = SyncState::Continuous;
        tracing::info!("初始同步完成, 耗时 {}ms", start.elapsed().as_millis());

        Ok(SyncSummary {
            duration_ms: start.elapsed().as_millis() as u64,
            ..summary
        })
    }
}
