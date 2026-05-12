use crate::api_client::ApiClient;
use crate::conflict_resolver::ConflictResolver;
use crate::errors::{Result, SyncError};
use crate::event_handler::EventHandler;
use crate::fs_scanner::FsScanner;
use crate::models::*;
use crate::sync_db::SyncDb;
use crate::transfer::TransferManager;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

pub struct SyncEngine {
    state: SyncState,
    db: Arc<SyncDb>,
    api: Arc<ApiClient>,
    transfer: Arc<TransferManager>,
    conflict: ConflictResolver,
    config: SyncConfig,
    sync_root_id: Option<String>,
    shutdown_token: CancellationToken,
}

impl SyncEngine {
    pub async fn new(config: SyncConfig) -> Result<Self> {
        let db_path = config.local_root.join(".sync_db.sqlite3");
        let db = Arc::new(SyncDb::open(&db_path)?);

        let api = Arc::new(ApiClient::new(&config.base_url, &config.access_token));

        let transfer_config = TransferConfig {
            max_concurrent: config.max_concurrent_transfers,
            bandwidth_limit: config.bandwidth_limit,
            ..Default::default()
        };
        let transfer = Arc::new(TransferManager::new(db.clone(), api.clone(), transfer_config));

        let conflict = ConflictResolver::new(config.conflict_strategy.clone());

        let sync_root_id = db.upsert_sync_root(&config).await.ok();

        Ok(Self {
            state: SyncState::Idle,
            db,
            api,
            transfer,
            conflict,
            config,
            sync_root_id,
            shutdown_token: CancellationToken::new(),
        })
    }

    /// 初始全量同步
    pub async fn run_initial_sync(&self) -> Result<SyncSummary> {
        let start = Instant::now();
        let scanner = FsScanner::new();

        tracing::info!("开始扫描本地文件系统: {}", self.config.local_root.display());
        let local_files = scanner.scan(&self.config.local_root, 50, false).await?;
        tracing::info!("本地扫描完成: {} 个条目", local_files.len());

        tracing::info!("开始扫描远程文件树: {}", self.config.remote_root);
        let remote_files = self.api.list_all_files(&self.config.remote_root).await?;
        tracing::info!("远程扫描完成: {} 个条目", remote_files.len());

        // Phase 2 将实现完整的差异计算和同步执行
        tracing::info!("初始同步完成 (占位实现)");

        Ok(SyncSummary {
            duration_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        })
    }

    /// 持续同步
    /// Phase 3 将实现完整的双事件源驱动 + 事件推送
    pub async fn run_continuous(&self) -> Result<()> {
        let _event_handler = EventHandler::new(
            self.api.clone(),
            uuid::Uuid::new_v4().to_string(),
        );

        // Phase 3: 实现完整的 SSE 订阅和本地文件监听
        loop {
            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    // 占位：定期轮询
                    tracing::debug!("持续同步心跳...");
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.shutdown_token.cancel();
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        Ok(())
    }

    pub async fn force_sync(&self) -> Result<SyncSummary> {
        self.run_initial_sync().await
    }

    pub fn status(&self) -> SyncStatusSnapshot {
        SyncStatusSnapshot {
            state: self.state.clone(),
            synced_files: 0,
            total_files: 0,
            uploading_count: 0,
            downloading_count: 0,
            conflict_count: 0,
            error_count: 0,
            last_sync_time: None,
            error_message: None,
        }
    }

    pub fn config(&self) -> SyncConfig {
        self.config.clone()
    }

    pub async fn update_config(&self, _config: SyncConfig) -> Result<()> {
        Ok(())
    }

    pub async fn update_access_token(&self, token: String) {
        self.api.update_token(token).await;
    }

    pub async fn shutdown(self) -> Result<()> {
        self.shutdown_token.cancel();
        Ok(())
    }

    pub async fn hydrate_file(&self, _local_path: &str) -> Result<()> {
        Ok(())
    }

    pub async fn sync_album(
        &self,
        album_paths: Vec<String>,
        remote_dcim_uri: &str,
    ) -> Result<()> {
        let synced = self.db.get_album_sync_records().await?;

        let new_photos: Vec<_> = album_paths.iter()
            .filter(|p| !synced.contains_key(*p))
            .collect();

        for photo_path in &new_photos {
            let _file_name = Path::new(photo_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Phase 5 将实现上传逻辑
            tracing::info!("上传照片: {}", photo_path);
        }

        Ok(())
    }

    pub async fn check_album_dirs(&self, base_uri: &str) -> Result<CloudAlbumCheckResult> {
        let files = self.api.list_files_page(base_uri, 0, 200, None).await?;

        let dcim_exists = files.files.iter().any(|f| f.name == "DCIM" && f.is_dir);
        let pictures_exists = files.files.iter().any(|f| f.name == "Pictures" && f.is_dir);

        Ok(CloudAlbumCheckResult {
            dcim_exists,
            pictures_exists,
            dcim_uri: if dcim_exists { Some(format!("{}/DCIM", base_uri)) } else { None },
            pictures_uri: if pictures_exists { Some(format!("{}/Pictures", base_uri)) } else { None },
        })
    }

    pub async fn create_album_dirs(&self, base_uri: &str) -> Result<()> {
        self.api.create_directory(base_uri, "DCIM").await?;
        self.api.create_directory(base_uri, "Pictures").await?;
        Ok(())
    }
}
