use crate::api_client::ApiClient;
use crate::conflict_resolver::ConflictResolver;
use crate::errors::Result;
use crate::event_handler::EventHandler;
use crate::event_sink::EventSink;
use crate::file_lock::FileLockRegistry;
use crate::fs_scanner::FsScanner;
use crate::models::*;
use crate::sync_db::SyncDb;
use crate::worker::WorkerPool;
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct SyncEngine {
    state: RwLock<SyncState>,
    db: Arc<SyncDb>,
    api: Arc<ApiClient>,
    config: RwLock<SyncConfig>,
    conflict: RwLock<ConflictResolver>,
    sync_root_id: Option<String>,
    shutdown_token: CancellationToken,
    worker_pool: WorkerPool,
    #[allow(dead_code)]
    file_locks: Arc<FileLockRegistry>,
    #[allow(dead_code)]
    ensured_dirs: Arc<DashMap<String, ()>>,
    event_sink: Arc<EventSink>,
}

impl SyncEngine {
    pub async fn new(config: SyncConfig) -> Result<Self> {
        let db_path = config.data_dir.join("sync_core").join("datas").join(".sync_db.sqlite3");
        let db_path_clone = db_path.clone();
        let db = Arc::new(tokio::task::spawn_blocking(move || SyncDb::open(&db_path_clone)).await??);

        let api = Arc::new(ApiClient::new(&config.base_url, &config.access_token, &config.refresh_token));

        let conflict = ConflictResolver::new(config.conflict_strategy.clone());

        let sync_root_id = match db.upsert_sync_root(&config).await {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!("写入 sync_root 失败: {}", e);
                None
            }
        };

        let shutdown_token = CancellationToken::new();
        let file_locks = Arc::new(FileLockRegistry::new());
        let ensured_dirs = Arc::new(DashMap::new());
        let event_sink = Arc::new(EventSink::new());

        let worker_pool = WorkerPool::new(
            db.clone(),
            api.clone(),
            file_locks.clone(),
            ensured_dirs.clone(),
            event_sink.clone(),
            shutdown_token.clone(),
        );

        Ok(Self {
            state: RwLock::new(SyncState::Idle),
            db,
            api,
            config: RwLock::new(config),
            conflict: RwLock::new(conflict),
            sync_root_id,
            shutdown_token,
            worker_pool,
            file_locks,
            ensured_dirs,
            event_sink,
        })
    }

    /// 从 RwLock<SyncConfig> 快照 WorkerConfig
    async fn snapshot_worker_config(&self) -> WorkerConfig {
        let config = self.config.read().await;
        WorkerConfig {
            local_root: config.local_root.clone(),
            remote_root: config.remote_root.clone(),
            max_concurrent_transfers: config.max_concurrent_transfers,
            bandwidth_limit: config.bandwidth_limit,
            conflict_strategy: config.conflict_strategy.clone(),
            sync_root_id: self.sync_root_id.clone().unwrap_or_default(),
        }
    }

    /// 初始全量同步
    pub async fn run_initial_sync(&self) -> Result<SyncSummary> {
        let start = Instant::now();
        *self.state.write().await = SyncState::Initializing;

        let (local_root, remote_root) = {
            let config = self.config.read().await;
            (config.local_root.clone(), config.remote_root.clone())
        };

        // 1. 扫描本地文件系统
        let scanner = FsScanner::new();
        tracing::info!("开始扫描本地文件系统: {}", local_root.display());
        let local_files = scanner.scan(&local_root, 50, false).await?;
        tracing::info!("本地扫描完成: {} 个条目", local_files.len());

        // 2. 扫描远程文件树
        tracing::info!("开始扫描远程文件树: {}", remote_root);
        let remote_files = self.api.list_all_files(&remote_root).await?;
        tracing::info!("远程扫描完成: {} 个条目", remote_files.len());

        // 3. 计算三路差异
        let db_mappings = self.load_all_mappings().await?;
        let plan = crate::diff::compute_diff(&local_files, &remote_files, &db_mappings, &remote_root);
        tracing::info!(
            "差异计算完成: 上传={}, 下载={}, 删本地={}, 删远程={}, 冲突={}",
            plan.uploads.len(),
            plan.downloads.len(),
            plan.delete_local.len(),
            plan.delete_remote.len(),
            plan.conflicts.len(),
        );

        // 4. 更新状态
        *self.state.write().await = SyncState::InitialSync {
            progress: InitialSyncProgress {
                scanned_local: local_files.len() as u64,
                scanned_remote: remote_files.len() as u64,
                total_to_sync: plan.total_actions(),
                ..Default::default()
            },
        };

        // 5. 提交到 WorkerPool
        let worker_config = self.snapshot_worker_config().await;
        let conflict_resolver = self.conflict.read().await.clone();

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

    /// 持续同步：双事件源驱动 (SSE + 本地文件监听)
    pub async fn run_continuous(&self) -> Result<()> {
        let event_handler = EventHandler::new(
            self.api.clone(),
            uuid::Uuid::new_v4().to_string(),
        );

        let (local_root, remote_root) = {
            let config = self.config.read().await;
            (config.local_root.clone(), config.remote_root.clone())
        };

        // 订阅远程 SSE 事件
        let mut remote_rx = event_handler.subscribe_sse(&remote_root).await?;

        // 启动本地文件监听 (notify)
        let (local_tx, mut local_rx) = tokio::sync::mpsc::channel::<LocalFileEvent>(256);
        let shutdown_clone = self.shutdown_token.clone();
        let watch_root = local_root.clone();

        std::thread::spawn(move || {
            use notify::{RecommendedWatcher, RecursiveMode, Event, EventKind};
            use notify::Watcher;

            let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

            let mut watcher = match RecommendedWatcher::new(
                move |res: notify::Result<Event>| { let _ = notify_tx.send(res); },
                notify::Config::default().with_poll_interval(std::time::Duration::from_secs(2)),
            ) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("无法启动文件监听: {}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&watch_root, RecursiveMode::Recursive) {
                tracing::error!("文件监听启动失败: {}", e);
                return;
            }

            tracing::info!("本地文件监听已启动: {}", watch_root.display());

            let mut created_buf: Vec<std::path::PathBuf> = Vec::new();
            let mut modified_buf: Vec<std::path::PathBuf> = Vec::new();
            let mut deleted_buf: Vec<std::path::PathBuf> = Vec::new();

            loop {
                if shutdown_clone.is_cancelled() {
                    break;
                }

                match notify_rx.recv_timeout(std::time::Duration::from_millis(500)) {
                    Ok(Ok(event)) => {
                        let paths: Vec<_> = event.paths.iter()
                            .filter(|p| {
                                !p.extension().map(|e| e == "sync_tmp").unwrap_or(false)
                            })
                            .cloned()
                            .collect();

                        if paths.is_empty() {
                            continue;
                        }

                        match event.kind {
                            EventKind::Create(_) => created_buf.extend(paths),
                            EventKind::Modify(_) => modified_buf.extend(paths),
                            EventKind::Remove(_) => deleted_buf.extend(paths),
                            _ => {}
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("文件监听错误: {}", e);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }

                if !created_buf.is_empty() {
                    let _ = local_tx.blocking_send(LocalFileEvent::Created(std::mem::take(&mut created_buf)));
                }
                if !modified_buf.is_empty() {
                    let _ = local_tx.blocking_send(LocalFileEvent::Modified(std::mem::take(&mut modified_buf)));
                }
                if !deleted_buf.is_empty() {
                    let _ = local_tx.blocking_send(LocalFileEvent::Deleted(std::mem::take(&mut deleted_buf)));
                }
            }

            let _ = watcher.unwatch(&watch_root);
            tracing::info!("本地文件监听已停止");
        });

        *self.state.write().await = SyncState::Continuous;
        tracing::info!("持续同步已启动");

        let mut debounce = crate::event_handler::EventDebouncer::new(
            std::time::Duration::from_millis(500),
        );

        loop {
            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
                    tracing::info!("持续同步收到停止信号");
                    break;
                }

                // 本地文件变化 → 空闲窗口批量收集 → 按事件类型分 Worker
                Some(event) = local_rx.recv() => {
                    // 空闲窗口收集：3秒内无新事件则视为批次结束
                    // 大文件复制中事件持续到达，窗口自然延长
                    let mut all_events = vec![event];
                    let idle_timeout = std::time::Duration::from_secs(3);
                    while let Ok(Some(e)) = tokio::time::timeout(idle_timeout, local_rx.recv()).await {
                        all_events.push(e)
                    }

                    // 按事件类型分类路径，同一路径去重（取最新事件）
                    let mut create_paths: std::collections::BTreeMap<String, std::path::PathBuf> = std::collections::BTreeMap::new();
                    let mut delete_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

                    for event in &all_events {
                        for path in event.paths() {
                            if !debounce.should_process(path) {
                                continue;
                            }

                            let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            if crate::fs_scanner::SKIP_NAMES.iter().any(|s| file_name == *s)
                                || file_name.starts_with(".sync_")
                                || crate::utils::is_conflict_file(&file_name) {
                                continue;
                            }

                            let relative = path.strip_prefix(&local_root)
                                .unwrap_or(path)
                                .to_string_lossy()
                                .to_string();
                            let relative = crate::utils::normalize_path(&relative);

                            match event {
                                LocalFileEvent::Created(_) | LocalFileEvent::Modified(_) => {
                                    // Created/Modified → 上传；如果之前被标记删除，则取消删除
                                    delete_paths.remove(relative.as_str());
                                    create_paths.insert(relative, path.clone());
                                }
                                LocalFileEvent::Deleted(_) => {
                                    if crate::utils::is_conflict_file(&relative) {
                                        continue;
                                    }
                                    // 如果之前被标记上传，则取消上传（文件已删）
                                    create_paths.remove(relative.as_str());
                                    delete_paths.insert(relative);
                                }
                            }
                        }
                    }
                    debounce.cleanup();

                    // === 提交上传任务 (Create/Modify) ===
                    if !create_paths.is_empty() {
                        let mut uploads = Vec::new();
                        let mut dir_paths: Vec<String> = Vec::new();

                        for (relative, path) in &create_paths {
                            if let Ok(metadata) = tokio::fs::metadata(path).await {
                                if !metadata.is_dir() && metadata.len() == 0 {
                                    continue;
                                }

                                if metadata.is_dir() {
                                    dir_paths.push(relative.clone());
                                    continue;
                                }

                                let size = metadata.len();
                                let quick_hash = crate::utils::quick_hash(path, size).await.unwrap_or_default();

                                let root_id = self.sync_root_id.clone().unwrap_or_default();
                                let db_mapping = self.db.get_file_mapping(&root_id, relative).await.ok().flatten();
                                if let Some(ref mapping) = db_mapping {
                                    if mapping.local_hash.as_deref() == Some(&quick_hash) {
                                        continue;
                                    }
                                }

                                let mtime_ms = metadata.modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_millis() as i64)
                                    .unwrap_or(0);
                                let mime_type = crate::fs_scanner::guess_mime_type(path);

                                uploads.push(SyncAction {
                                    relative_path: relative.clone(),
                                    local_entry: Some(LocalFileEntry {
                                        relative_path: std::path::PathBuf::from(relative),
                                        size,
                                        mtime_ms,
                                        quick_hash,
                                        is_dir: false,
                                        mime_type,
                                    }),
                                    remote_entry: None,
                                    db_mapping,
                                });
                            }
                        }

                        // 找出顶层目录，过滤掉被顶层目录包含的普通文件
                        let scan_dirs = find_top_level_dirs(&dir_paths);
                        if !scan_dirs.is_empty() {
                            uploads.retain(|action| {
                                !scan_dirs.iter().any(|dir| {
                                    action.relative_path.starts_with(dir)
                                        && action.relative_path.as_bytes().get(dir.len()) == Some(&b'/')
                                })
                            });
                        }

                        if !uploads.is_empty() || !scan_dirs.is_empty() {
                            tracing::info!(
                                "本地事件收集完成: 上传={}, 目录扫描={:?}",
                                uploads.len(), scan_dirs,
                            );
                            let plan = SyncPlan {
                                uploads,
                                scan_dirs,
                                ..Default::default()
                            };
                            let worker_config = self.snapshot_worker_config().await;
                            let conflict_resolver = self.conflict.read().await.clone();
                            self.worker_pool.submit_background(
                                plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                            ).await;
                        }
                    }

                    // === 提交删除任务 (Delete) ===
                    if !delete_paths.is_empty() {
                        let delete_local: Vec<SyncAction> = delete_paths.into_iter().map(|relative| {
                            tracing::info!("检测到本地文件删除: {}", relative);
                            SyncAction {
                                relative_path: relative,
                                local_entry: None,
                                remote_entry: None,
                                db_mapping: None,
                            }
                        }).collect();

                        tracing::info!("本地删除事件收集完成: 删除={}", delete_local.len());
                        let plan = SyncPlan {
                            delete_local,
                            ..Default::default()
                        };
                        let worker_config = self.snapshot_worker_config().await;
                        let conflict_resolver = self.conflict.read().await.clone();
                        self.worker_pool.submit_background(
                            plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                        ).await;
                    }
                }

                // 远程文件变化 → 构建 SyncPlan → submit_background
                Some(event) = remote_rx.recv() => {
                    let mut downloads = Vec::new();
                    let mut _delete_remote_actions: Vec<SyncAction> = Vec::new();

                    match &event {
                        RemoteFileEvent::Created(remote) | RemoteFileEvent::Modified(remote) => {
                            let relative = crate::diff::remote_relative_path(
                                &remote_root,
                                &remote.path,
                                &remote.name,
                                remote.is_dir,
                            );
                            tracing::debug!("远程下载: {}", relative);

                            downloads.push(SyncAction {
                                relative_path: relative.clone(),
                                local_entry: None,
                                remote_entry: Some(remote.clone()),
                                db_mapping: None,
                            });
                        }
                        RemoteFileEvent::Deleted { uri, name } => {
                            let relative = crate::diff::remote_relative_path(
                                &remote_root,
                                uri,
                                name,
                                false,
                            );
                            tracing::debug!("远程删除，删除本地: {}", relative);
                            let local_path = local_root.join(&relative);
                            let _ = tokio::fs::remove_file(&local_path).await;
                            continue;
                        }
                    }

                    if !downloads.is_empty() {
                        let plan = SyncPlan {
                            downloads,
                            ..Default::default()
                        };
                        let worker_config = self.snapshot_worker_config().await;
                        let conflict_resolver = self.conflict.read().await.clone();
                        self.worker_pool.submit_background(
                            plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                        ).await;
                    }
                }

                // 定期心跳
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                    tracing::debug!("持续同步心跳");
                    debounce.cleanup();
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
        *self.state.write().await = SyncState::Paused;
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        *self.state.write().await = SyncState::Continuous;
        Ok(())
    }

    pub async fn force_sync(&self) -> Result<SyncSummary> {
        self.run_initial_sync().await
    }

    pub fn status(&self) -> SyncStatusSnapshot {
        let state = self.state.try_read().map(|g| g.clone()).unwrap_or(SyncState::Idle);

        let (synced_files, total_files) = match &state {
            SyncState::InitialSync { progress } => {
                let done = progress.uploaded + progress.downloaded;
                (done, progress.total_to_sync)
            }
            _ => (0, 0),
        };

        SyncStatusSnapshot {
            state,
            synced_files,
            total_files,
            uploading_count: 0,
            downloading_count: 0,
            conflict_count: 0,
            error_count: 0,
            last_sync_time: None,
            error_message: None,
        }
    }

    pub fn active_worker_count(&self) -> u32 {
        self.worker_pool.active_worker_count() as u32
    }

    pub async fn config(&self) -> SyncConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: SyncConfig) -> Result<()> {
        let old_access_token = {
            let config = self.config.read().await;
            config.access_token.clone()
        };

        // 更新冲突策略
        *self.conflict.write().await = ConflictResolver::new(new_config.conflict_strategy.clone());

        // 更新 API token（如果变化）
        if new_config.access_token != old_access_token {
            self.api.update_token(new_config.access_token.clone()).await;
        }

        let new_bandwidth = new_config.bandwidth_limit;
        let new_conflict = format!("{:?}", new_config.conflict_strategy);
        let new_mode = format!("{:?}", new_config.sync_mode);
        let new_max_concurrent = new_config.max_concurrent_transfers;
        *self.config.write().await = new_config;

        if new_bandwidth.is_some() {
            tracing::info!("仅对下载限速生效, 由于Cloudreve实现原因, 上传限速无法生效");
        }
        tracing::info!(
            "同步配置已更新: 模式={}, 冲突策略={}, 并发={}, 带宽限制={:?}",
            new_mode, new_conflict, new_max_concurrent, new_bandwidth
        );
        Ok(())
    }

    pub async fn update_access_token(&self, token: String) {
        self.api.update_token(token).await;
    }

    pub async fn register_event_sink(&self, sink: crate::frb_generated::StreamSink<crate::api::ffi_types::SyncEventFfi>) {
        self.event_sink.register(sink).await;
    }

    // ===== 任务查询 =====

    pub async fn get_active_tasks(&self) -> Result<Vec<SyncTask>> {
        self.db.get_active_sync_tasks().await
    }

    pub async fn get_recent_tasks(&self, limit: u32) -> Result<Vec<SyncTask>> {
        self.db.get_recent_sync_tasks(limit).await
    }

    pub async fn get_task_detail(&self, task_id: &str) -> Result<Vec<SyncTaskItem>> {
        self.db.get_sync_task_items(task_id).await
    }

    pub async fn query_task_items(&self, filter: &TaskItemFilter) -> Result<Vec<SyncTaskItem>> {
        self.db.query_task_items(filter).await
    }

    // ===== 相册同步 =====

    pub async fn sync_album(
        &self,
        album_paths: Vec<String>,
        remote_dcim_uri: &str,
    ) -> Result<()> {
        let synced = self.db.get_album_sync_records().await?;

        let new_photos: Vec<_> = album_paths.iter()
            .filter(|p| !synced.contains_key(*p))
            .collect();

        let total = new_photos.len();
        if total == 0 {
            tracing::info!("相册同步: 无新照片");
            return Ok(());
        }

        tracing::info!("相册同步: 发现 {} 张新照片", total);

        for (i, photo_path) in new_photos.iter().enumerate() {
            let local_path = Path::new(photo_path);
            let file_name = local_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("photo_{}", i));

            match tokio::fs::metadata(photo_path).await {
                Ok(metadata) => {
                    let file_size = metadata.len();

                    match self.api.create_upload_session(remote_dcim_uri, file_size, false, None, None, None).await {
                        Ok(session) => {
                            match crate::uploader::upload_file_chunked(
                                &self.api, local_path, &session,
                            ).await {
                                Ok(_) => {
                                    let remote_uri = format!("{}/{}", remote_dcim_uri, file_name);
                                    let hash = crate::utils::quick_hash(local_path, file_size).await.unwrap_or_default();

                                    if let Err(e) = self.db.add_album_sync_record(
                                        photo_path,
                                        &remote_uri,
                                        &hash,
                                    ).await {
                                        tracing::warn!("记录同步状态失败: {}", e);
                                    }

                                    tracing::info!("照片上传完成 ({}/{}): {}", i + 1, total, file_name);
                                }
                                Err(e) => {
                                    tracing::error!("上传照片失败 {}: {}", file_name, e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("创建上传会话失败 {}: {}", file_name, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("无法读取照片元数据 {}: {}", photo_path, e);
                }
            }
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

    pub async fn hydrate_file(&self, _local_path: &str) -> Result<()> {
        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        self.shutdown_token.cancel();
        Ok(())
    }

    /// 从数据库加载当前所有 file_mapping
    async fn load_all_mappings(&self) -> Result<HashMap<String, FileMapping>> {
        let root_id = match &self.sync_root_id {
            Some(id) => id.clone(),
            None => return Ok(HashMap::new()),
        };

        let pool = self.db.read_pool();
        let result = tokio::task::spawn_blocking(move || -> Result<HashMap<String, FileMapping>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, sync_root_id, local_path, remote_uri, remote_file_id,
                        local_hash, remote_hash, local_mtime, remote_mtime,
                        local_size, remote_size, sync_status, is_placeholder
                 FROM file_mapping WHERE sync_root_id = ?1"
            )?;

            let mappings: HashMap<String, FileMapping> = stmt.query_map(
                rusqlite::params![root_id],
                |row| {
                    let local_path: String = row.get(2)?;
                    Ok((
                        crate::utils::normalize_path(&local_path),
                        FileMapping {
                            id: row.get(0)?,
                            sync_root_id: row.get(1)?,
                            local_path: std::path::PathBuf::from(local_path),
                            remote_uri: row.get(3)?,
                            remote_file_id: row.get(4)?,
                            local_hash: row.get(5)?,
                            remote_hash: row.get(6)?,
                            local_mtime: row.get(7)?,
                            remote_mtime: row.get(8)?,
                            local_size: row.get(9)?,
                            remote_size: row.get(10)?,
                            sync_status: crate::diff::parse_sync_status_from_str(&row.get::<_, String>(11)?),
                            is_placeholder: row.get::<_, i32>(12)? != 0,
                        },
                    ))
                },
            )?.filter_map(|r| r.ok()).collect();

            Ok(mappings)
        }).await??;

        Ok(result)
    }
}

/// 从目录路径列表中找出顶层目录（不被其他目录包含的）
fn find_top_level_dirs(dirs: &[String]) -> Vec<String> {
    if dirs.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<&String> = dirs.iter().collect();
    sorted.sort();

    let mut top_level = Vec::new();
    for dir in &sorted {
        let dominated = top_level.iter().any(|parent: &String| {
            dir.starts_with(parent.as_str())
                && dir.as_bytes().get(parent.len()) == Some(&b'/')
        });
        if !dominated {
            // 移除已有的子目录（当前目录更顶层）
            top_level.retain(|existing: &String| {
                !existing.starts_with(dir.as_str())
                    || existing.as_bytes().get(dir.len()) != Some(&b'/')
            });
            top_level.push((*dir).clone());
        }
    }

    top_level
}
