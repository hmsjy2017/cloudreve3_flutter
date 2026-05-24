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
    shutdown_token: std::sync::Mutex<CancellationToken>,
    worker_pool: WorkerPool,
    #[allow(dead_code)]
    file_locks: Arc<FileLockRegistry>,
    #[allow(dead_code)]
    ensured_dirs: Arc<DashMap<String, ()>>,
    event_sink: Arc<EventSink>,
    /// 远程操作导致的本地路径变化，抑制本地 debouncer 自触发事件
    suppress_paths: Arc<DashMap<String, std::time::Instant>>,
}

impl SyncEngine {
    pub async fn new(config: SyncConfig) -> Result<Self> {
        let db_path = config.data_dir.join("sync_core").join("datas").join(".sync_db.sqlite3");
        let db_path_clone = db_path.clone();
        let db = Arc::new(tokio::task::spawn_blocking(move || SyncDb::open(&db_path_clone)).await??);

        let api = Arc::new(ApiClient::new(&config.base_url, &config.access_token, &config.refresh_token, &config.client_id));

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
        let suppress_paths = Arc::new(DashMap::new());

        let max_workers = config.max_workers;
        let client_id = config.client_id.clone();
        let worker_pool = WorkerPool::new(
            db.clone(),
            api.clone(),
            file_locks.clone(),
            ensured_dirs.clone(),
            event_sink.clone(),
            shutdown_token.clone(),
            max_workers,
            &client_id,
        );

        Ok(Self {
            state: RwLock::new(SyncState::Idle),
            db,
            api,
            config: RwLock::new(config),
            conflict: RwLock::new(conflict),
            sync_root_id,
            shutdown_token: std::sync::Mutex::new(shutdown_token),
            worker_pool,
            file_locks,
            ensured_dirs,
            event_sink,
            suppress_paths,
        })
    }

    async fn snapshot_worker_config(&self) -> WorkerConfig {
        let config = self.config.read().await;
        WorkerConfig {
            local_root: config.local_root.clone(),
            remote_root: config.remote_root.clone(),
            max_concurrent_transfers: config.max_concurrent_transfers,
            bandwidth_limit: config.bandwidth_limit,
            conflict_strategy: config.conflict_strategy.clone(),
            sync_root_id: self.sync_root_id.clone().unwrap_or_default(),
            sync_mode: config.sync_mode.clone(),
        }
    }

    /// 初始全量同步
    pub async fn run_initial_sync(&self) -> Result<SyncSummary> {
        let start = Instant::now();

        // 重置 shutdown token，确保可重新启动
        let new_token = CancellationToken::new();
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

    /// 持续同步：双事件源驱动 (SSE + 本地文件监听)，按 sync_mode 选择事件源
    pub async fn run_continuous(&self) -> Result<()> {
        let event_handler = EventHandler::new(
            self.api.clone(),
            self.api.client_id().to_string(),
        );

        let (local_root, remote_root, sync_mode) = {
            let config = self.config.read().await;
            (config.local_root.clone(), config.remote_root.clone(), config.sync_mode.clone())
        };

        // 仅 DownloadOnly 和 Full 订阅 SSE
        let mut remote_rx = if matches!(sync_mode, SyncMode::DownloadOnly | SyncMode::Full) {
            Some(event_handler.subscribe_sse(&remote_root).await?)
        } else {
            tracing::info!("仅上传模式: 不订阅 SSE 远程事件");
            None
        };

        // 仅 UploadOnly 和 Full 启动本地文件监听
        let mut local_rx = if matches!(sync_mode, SyncMode::UploadOnly | SyncMode::Full) {
            let (local_tx, rx) = tokio::sync::mpsc::channel::<LocalFileEvent>(256);
            let shutdown_clone = self.shutdown_token.lock().unwrap().clone();
            let watch_root = local_root.clone();

            std::thread::spawn(move || {
                use notify_debouncer_full::notify::{RecursiveMode, EventKind};
                use notify_debouncer_full::notify::event::{ModifyKind, RenameMode};
                use notify_debouncer_full::new_debouncer;

                let tx = local_tx.clone();
                let shutdown = shutdown_clone.clone();

                let mut debouncer = match new_debouncer(
                    std::time::Duration::from_millis(500),
                    None,
                    move |result: notify_debouncer_full::DebounceEventResult| {
                        match result {
                            Ok(events) => {
                                for event in events {
                                    if shutdown.is_cancelled() { return; }
                                    let kind = event.kind;
                                    let paths = &event.paths;

                                    let filtered: Vec<_> = paths.iter()
                                        .filter(|p| !p.extension().map(|e| e == "sync_tmp").unwrap_or(false))
                                        .cloned()
                                        .collect();
                                    if filtered.is_empty() { continue; }

                                    match kind {
                                        EventKind::Create(_) => {
                                            let _ = tx.blocking_send(LocalFileEvent::Created(filtered));
                                        }
                                        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {}
                                        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {}
                                        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                                            if filtered.len() == 2 {
                                                let _ = tx.blocking_send(LocalFileEvent::Renamed {
                                                    old_paths: vec![filtered[0].clone()],
                                                    new_paths: vec![filtered[1].clone()],
                                                });
                                            }
                                        }
                                        EventKind::Modify(ModifyKind::Name(RenameMode::Other)) => {
                                            let _ = tx.blocking_send(LocalFileEvent::Modified(filtered));
                                        }
                                        EventKind::Modify(_) => {
                                            let _ = tx.blocking_send(LocalFileEvent::Modified(filtered));
                                        }
                                        EventKind::Remove(_) => {
                                            let _ = tx.blocking_send(LocalFileEvent::Deleted(filtered));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Err(errors) => {
                                for e in errors {
                                    tracing::warn!("文件监听去抖错误: {}", e);
                                }
                            }
                        }
                    },
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::error!("无法启动文件监听: {}", e);
                        return;
                    }
                };

                if let Err(e) = debouncer.watch(&watch_root, RecursiveMode::Recursive) {
                    tracing::error!("文件监听启动失败: {}", e);
                    return;
                }

                tracing::info!("本地文件监听已启动(debouncer): {}", watch_root.display());

                while !shutdown_clone.is_cancelled() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

                let _ = debouncer.unwatch(&watch_root);
                tracing::info!("本地文件监听已停止");
            });

            Some(rx)
        } else {
            tracing::info!("仅下载模式: 不启动本地文件监听");
            None
        };

        *self.state.write().await = SyncState::Continuous;
        tracing::info!("持续同步已启动, 模式={:?}", sync_mode);

        let mut debounce = crate::event_handler::EventDebouncer::new(
            std::time::Duration::from_millis(500),
        );

        let shutdown_token = self.shutdown_token.lock().unwrap().clone();
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("持续同步收到停止信号");
                    break;
                }

                // 本地文件变化（仅 UploadOnly / Full）
                Some(event) = async {
                    match &mut local_rx {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    let mut all_events = vec![event];
                    let idle_timeout = std::time::Duration::from_secs(3);
                    if let Some(rx) = &mut local_rx {
                        while let Ok(Some(e)) = tokio::time::timeout(idle_timeout, rx.recv()).await {
                            all_events.push(e)
                        }
                    }

                    self.handle_local_events(all_events, &local_root, &mut debounce).await;
                }

                // 远程文件变化（仅 DownloadOnly / Full）
                Some(event) = async {
                    match &mut remote_rx {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    self.handle_remote_event(event, &local_root, &remote_root).await;
                }

                // 定期心跳
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                    tracing::trace!("持续同步心跳");
                    debounce.cleanup();
                }
            }
        }

        Ok(())
    }

    /// 处理本地事件批次
    async fn handle_local_events(
        &self,
        all_events: Vec<LocalFileEvent>,
        local_root: &std::path::Path,
        debounce: &mut crate::event_handler::EventDebouncer,
    ) {
        // DownloadOnly: 忽略所有本地事件
        let sync_mode = {
            let config = self.config.read().await;
            config.sync_mode.clone()
        };
        if matches!(sync_mode, SyncMode::DownloadOnly) {
            return;
        }

        let root_id = self.sync_root_id.clone().unwrap_or_default();

        // 清理过期的 suppress 记录（超过 30 秒）
        let now = std::time::Instant::now();
        self.suppress_paths.retain(|_, ts| now.duration_since(*ts).as_secs() < 30);

        // === 第一步：提取 Renamed/Moved 事件，查 DB 构建操作 ===
        let mut rename_remote: Vec<RenameAction> = Vec::new();
        let mut move_remote: Vec<MoveAction> = Vec::new();
        let mut handled_old_rels: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut handled_new_rels: std::collections::HashSet<String> = std::collections::HashSet::new();

        for event in &all_events {
            match event {
                LocalFileEvent::Renamed { old_paths, new_paths } => {
                    for (old_path, new_path) in old_paths.iter().zip(new_paths.iter()) {
                        if let Some((old_rel, new_rel)) = rel_pair(local_root, old_path, new_path) {
                            // 被远程操作抑制的 rename，跳过
                            if self.suppress_paths.contains_key(&old_rel) || self.suppress_paths.contains_key(&new_rel) {
                                tracing::trace!("本地重命名被抑制(远程操作导致): {} -> {}", old_rel, new_rel);
                                continue;
                            }
                            let new_name = new_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();

                            if let Ok(Some(mapping)) = self.db.get_file_mapping(&root_id, &old_rel).await {
                                tracing::info!("检测到本地重命名: {} -> {}", old_rel, new_rel);
                                rename_remote.push(RenameAction {
                                    old_relative_path: old_rel.clone(),
                                    new_relative_path: new_rel.clone(),
                                    remote_uri: mapping.remote_uri.clone(),
                                    new_name,
                                });
                                handled_old_rels.insert(old_rel);
                                handled_new_rels.insert(new_rel);
                            } else {
                                tracing::info!("本地重命名但旧路径无DB映射，按新建处理: {} -> {}", old_rel, new_rel);
                            }
                        }
                    }
                }
                LocalFileEvent::Moved { old_paths, new_paths } => {
                    for (old_path, new_path) in old_paths.iter().zip(new_paths.iter()) {
                        if let Some((old_rel, new_rel)) = rel_pair(local_root, old_path, new_path) {
                            // 被远程操作抑制的 move，跳过
                            if self.suppress_paths.contains_key(&old_rel) || self.suppress_paths.contains_key(&new_rel) {
                                tracing::trace!("本地移动被抑制(远程操作导致): {} -> {}", old_rel, new_rel);
                                continue;
                            }
                            if let Ok(Some(mapping)) = self.db.get_file_mapping(&root_id, &old_rel).await {
                                // 计算目标远程目录 URI
                                let remote_root = { self.config.read().await.remote_root.clone() };
                                let new_rel_path = std::path::PathBuf::from(&new_rel);
                                let dst_dir_rel = new_rel_path.parent()
                                    .map(|p| crate::utils::normalize_path(&p.to_string_lossy()))
                                    .unwrap_or_default();
                                let dst_remote_dir_uri = format!("{}/{}",
                                    remote_root.trim_end_matches('/'),
                                    dst_dir_rel.trim_start_matches('/'));

                                tracing::info!("检测到本地移动: {} -> {}", old_rel, new_rel);
                                move_remote.push(MoveAction {
                                    old_relative_path: old_rel.clone(),
                                    new_relative_path: new_rel.clone(),
                                    remote_uri: mapping.remote_uri.clone(),
                                    dst_remote_dir_uri,
                                });
                                handled_old_rels.insert(old_rel);
                                handled_new_rels.insert(new_rel);
                            } else {
                                tracing::info!("本地移动但旧路径无DB映射，按新建处理: {} -> {}", old_rel, new_rel);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // === 第二步：按事件类型分类路径，跳过已识别为 rename/move 的路径 ===
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

                let relative = path.strip_prefix(local_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                let relative = crate::utils::normalize_path(&relative);

                // 跳过已识别为 rename/move 的路径，或被远程操作抑制的路径
                if handled_old_rels.contains(&relative) || handled_new_rels.contains(&relative)
                    || self.suppress_paths.contains_key(&relative) {
                    continue;
                }

                match event {
                    LocalFileEvent::Created(_) | LocalFileEvent::Modified(_) => {
                        delete_paths.remove(relative.as_str());
                        create_paths.insert(relative, path.clone());
                    }
                    LocalFileEvent::Deleted(_) => {
                        if crate::utils::is_conflict_file(&relative) {
                            continue;
                        }
                        create_paths.remove(relative.as_str());
                        delete_paths.insert(relative);
                    }
                    LocalFileEvent::Renamed { .. } | LocalFileEvent::Moved { .. } => {}
                }
            }
        }
        debounce.cleanup();

        // === hash 匹配回退：检测 delete+create 为 rename 的情况 ===
        if !delete_paths.is_empty() && !create_paths.is_empty() {
            let mut matched_deletes: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut matched_creates: std::collections::HashSet<String> = std::collections::HashSet::new();

            for (new_rel, new_path) in &create_paths {
                if let Ok(metadata) = tokio::fs::metadata(new_path).await {
                    if metadata.is_dir() || metadata.len() == 0 { continue; }
                    let new_hash = crate::utils::quick_hash(new_path, metadata.len()).await.unwrap_or_default();
                    if new_hash.is_empty() { continue; }

                    for del_rel in &delete_paths {
                        if matched_deletes.contains(del_rel.as_str()) { continue; }
                        if let Ok(Some(mapping)) = self.db.get_file_mapping(&root_id, del_rel).await {
                            if mapping.local_hash.as_deref() == Some(&new_hash) {
                                let new_name = new_path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();

                                // 判断同目录(=rename) 还是跨目录(=move)
                                let old_dir = std::path::PathBuf::from(del_rel).parent()
                                    .map(|p| crate::utils::normalize_path(&p.to_string_lossy()))
                                    .unwrap_or_default();
                                let new_dir = std::path::PathBuf::from(new_rel.as_str()).parent()
                                    .map(|p| crate::utils::normalize_path(&p.to_string_lossy()))
                                    .unwrap_or_default();

                                if old_dir == new_dir {
                                    tracing::info!("hash匹配检测到重命名: {} -> {}", del_rel, new_rel);
                                    rename_remote.push(RenameAction {
                                        old_relative_path: del_rel.clone(),
                                        new_relative_path: new_rel.clone(),
                                        remote_uri: mapping.remote_uri.clone(),
                                        new_name,
                                    });
                                } else {
                                    let remote_root = { self.config.read().await.remote_root.clone() };
                                    let dst_remote_dir_uri = format!("{}/{}",
                                        remote_root.trim_end_matches('/'),
                                        new_dir.trim_start_matches('/'));
                                    tracing::info!("hash匹配检测到移动: {} -> {}", del_rel, new_rel);
                                    move_remote.push(MoveAction {
                                        old_relative_path: del_rel.clone(),
                                        new_relative_path: new_rel.clone(),
                                        remote_uri: mapping.remote_uri.clone(),
                                        dst_remote_dir_uri,
                                    });
                                }
                                matched_deletes.insert(del_rel.clone());
                                matched_creates.insert(new_rel.clone());
                                break;
                            }
                        }
                    }
                }
            }

            create_paths.retain(|rel, _| !matched_creates.contains(rel.as_str()));
            delete_paths.retain(|rel| !matched_deletes.contains(rel.as_str()));
        }

        // === 提交重命名任务 ===
        if !rename_remote.is_empty() {
            let plan = SyncPlan {
                rename_remote,
                ..Default::default()
            };
            let worker_config = self.snapshot_worker_config().await;
            let conflict_resolver = self.conflict.read().await.clone();
            self.worker_pool.submit_background(
                plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
            ).await;
        }

        // === 提交移动任务 ===
        if !move_remote.is_empty() {
            let plan = SyncPlan {
                move_remote,
                ..Default::default()
            };
            let worker_config = self.snapshot_worker_config().await;
            let conflict_resolver = self.conflict.read().await.clone();
            self.worker_pool.submit_background(
                plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
            ).await;
        }

        // === 提交上传任务 (Create/Modify) ===
        if !create_paths.is_empty() {
            let mut uploads = Vec::new();
            let mut dir_paths: Vec<String> = Vec::new();
            let mut skipped_unstable = 0usize;
            let mut skipped_uploading = 0usize;

            for (relative, path) in &create_paths {
                if let Ok(metadata) = tokio::fs::metadata(path).await {
                    if !metadata.is_dir() && metadata.len() == 0 {
                        continue;
                    }

                    if metadata.is_dir() {
                        dir_paths.push(relative.clone());
                        continue;
                    }

                    // 去重：跳过已在上传队列中的文件
                    if self.worker_pool.is_uploading(relative) {
                        skipped_uploading += 1;
                        tracing::debug!("文件正在上传中，跳过: {}", relative);
                        continue;
                    }

                    // 文件稳定性检测：尝试独占读取，失败说明文件仍在被写入（如大文件复制中）
                    if !is_file_stable(path).await {
                        skipped_unstable += 1;
                        tracing::debug!("文件尚未稳定（可能正在写入），跳过: {}", relative);
                        continue;
                    }

                    let size = metadata.len();
                    let quick_hash = crate::utils::quick_hash(path, size).await.unwrap_or_default();

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

            let scan_dirs = find_top_level_dirs(&dir_paths);

            // 过滤掉包含已处理 rename/move 文件的目录，以及被抑制的目录
            let all_handled: std::collections::HashSet<&String> = handled_old_rels.iter()
                .chain(handled_new_rels.iter())
                .collect();
            let filtered_scan_dirs: Vec<String> = scan_dirs.into_iter().filter(|dir| {
                // 被 rename/move 处理过的文件所在目录，跳过
                if all_handled.iter().any(|rel| {
                    rel.starts_with(dir.as_str())
                        && rel.as_bytes().get(dir.len()) == Some(&b'/')
                }) {
                    return false;
                }
                // 被 suppress 的路径所在目录，跳过（远程操作导致的本地变更）
                for entry in self.suppress_paths.iter() {
                    let rel = entry.key();
                    if rel.starts_with(dir.as_str())
                        && rel.as_bytes().get(dir.len()) == Some(&b'/') {
                        return false;
                    }
                    // 目录自身被抑制
                    if dir.as_str() == rel.as_str() {
                        return false;
                    }
                }
                true
            }).collect();

            if !filtered_scan_dirs.is_empty() {
                uploads.retain(|action| {
                    !filtered_scan_dirs.iter().any(|dir| {
                        action.relative_path.starts_with(dir)
                            && action.relative_path.as_bytes().get(dir.len()) == Some(&b'/')
                    })
                });
            }

            if !uploads.is_empty() || !filtered_scan_dirs.is_empty() {
                tracing::info!(
                    "本地事件收集完成: 上传={}, 目录扫描={:?}, 跳过(未稳定)={}, 跳过(重复上传)={}",
                    uploads.len(), filtered_scan_dirs,
                    skipped_unstable, skipped_uploading,
                );
                let plan = SyncPlan {
                    uploads,
                    scan_dirs: filtered_scan_dirs,
                    ..Default::default()
                };
                let worker_config = self.snapshot_worker_config().await;
                let conflict_resolver = self.conflict.read().await.clone();
                self.worker_pool.submit_background(
                    plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                ).await;
            }
        }

        // === 提交删除远程任务 (本地删除 → 删除远程，仅 Full 模式) ===
        if !delete_paths.is_empty() && matches!(sync_mode, SyncMode::Full) {
            let mut delete_remote: Vec<SyncAction> = Vec::new();
            for relative in &delete_paths {
                tracing::info!("检测到本地文件删除: {}", relative);
                if let Ok(Some(mapping)) = self.db.get_file_mapping(&root_id, relative).await {
                    delete_remote.push(SyncAction {
                        relative_path: relative.clone(),
                        local_entry: None,
                        remote_entry: Some(RemoteFileEntry {
                            uri: mapping.remote_uri.clone(),
                            name: String::new(),
                            size: 0,
                            mtime_ms: 0,
                            hash: None,
                            is_dir: false,
                            file_id: mapping.remote_file_id.clone(),
                            path: String::new(),
                            created_at_ms: 0,
                        }),
                        db_mapping: Some(mapping),
                    });
                    let _ = self.db.delete_file_mapping(&root_id, relative).await;
                }
            }

            if !delete_remote.is_empty() {
                tracing::info!("本地删除事件收集完成: 删远程={}", delete_remote.len());
                let plan = SyncPlan {
                    delete_remote,
                    ..Default::default()
                };
                let worker_config = self.snapshot_worker_config().await;
                let conflict_resolver = self.conflict.read().await.clone();
                self.worker_pool.submit_background(
                    plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                ).await;
            }
        }

        // UploadOnly 模式下本地删除仅清理 DB mapping，不删除远程
        if !delete_paths.is_empty() && matches!(sync_mode, SyncMode::UploadOnly) {
            for relative in &delete_paths {
                let _ = self.db.delete_file_mapping(&root_id, relative).await;
                tracing::debug!("仅上传模式: 本地删除仅清理映射，不删除远程: {}", relative);
            }
        }
    }

    /// 处理远程事件
    async fn handle_remote_event(
        &self,
        event: RemoteFileEvent,
        local_root: &std::path::Path,
        remote_root: &str,
    ) {
        // UploadOnly: 忽略所有远程事件
        let sync_mode = {
            let config = self.config.read().await;
            config.sync_mode.clone()
        };
        if matches!(sync_mode, SyncMode::UploadOnly) {
            return;
        }

        let root_id = self.sync_root_id.clone().unwrap_or_default();

        match &event {
            RemoteFileEvent::Created(remote) | RemoteFileEvent::Modified(remote) => {
                let relative = crate::diff::remote_relative_path(
                    remote_root,
                    &remote.path,
                    &remote.name,
                    remote.is_dir,
                );
                tracing::info!("[远程事件] {}/{:?}: {}", event_type_name(&event), remote.file_id, relative);

                let remote_entry = if remote.size == 0 && !remote.is_dir {
                    match self.api.get_file_info(&remote.uri).await {
                        Ok(info) => {
                            tracing::debug!("[远程事件] 获取文件详情成功: {} ({}bytes)", relative, info.size);
                            info
                        }
                        Err(e) => {
                            tracing::warn!("[远程事件] 获取文件详情失败: {}: {}", relative, e);
                            remote.clone()
                        }
                    }
                } else {
                    remote.clone()
                };

                // 抑制本地 debouncer 检测到下载文件的自触发事件
                let now = std::time::Instant::now();
                self.suppress_paths.insert(relative.clone(), now);
                if let Some(parent) = std::path::PathBuf::from(&relative).parent() {
                    let parent_rel = crate::utils::normalize_path(&parent.to_string_lossy());
                    if !parent_rel.is_empty() {
                        self.suppress_paths.insert(parent_rel, now);
                    }
                }

                let plan = SyncPlan {
                    downloads: vec![SyncAction {
                        relative_path: relative,
                        local_entry: None,
                        remote_entry: Some(remote_entry),
                        db_mapping: None,
                    }],
                    ..Default::default()
                };
                let worker_config = self.snapshot_worker_config().await;
                let conflict_resolver = self.conflict.read().await.clone();
                self.worker_pool.submit_background(
                    plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                ).await;
            }
            RemoteFileEvent::Deleted { uri, name } => {
                let relative = crate::diff::remote_relative_path(
                    remote_root,
                    uri,
                    name,
                    false,
                );
                tracing::info!("[远程事件] 删除: {}", relative);

                // 删除本地文件
                let local_path = local_root.join(&relative);
                if local_path.exists() {
                    if local_path.is_dir() {
                        let _ = tokio::fs::remove_dir_all(&local_path).await;
                    } else {
                        let _ = tokio::fs::remove_file(&local_path).await;
                    }
                    tracing::info!("[远程事件] 已删除本地文件: {}", relative);
                }
                let _ = self.db.delete_file_mapping(&root_id, &relative).await;
                // 抑制本地 debouncer 检测到的删除事件
                self.suppress_paths.insert(relative.clone(), std::time::Instant::now());
            }
            RemoteFileEvent::Renamed { old_uri, new_entry } => {
                let old_relative = crate::diff::remote_relative_path(
                    remote_root,
                    old_uri,
                    &new_entry.name,
                    false,
                );
                let new_relative = crate::diff::remote_relative_path(
                    remote_root,
                    &new_entry.path,
                    &new_entry.name,
                    new_entry.is_dir,
                );
                tracing::info!("[远程事件] 重命名: {} -> {}", old_relative, new_relative);

                // 抑制本地 debouncer 自触发事件
                let now = std::time::Instant::now();
                self.suppress_paths.insert(old_relative.clone(), now);
                self.suppress_paths.insert(new_relative.clone(), now);

                let old_local_path = local_root.join(&old_relative);

                if old_local_path.exists() {
                    let plan = SyncPlan {
                        rename_local: vec![LocalRenameAction {
                            old_relative_path: old_relative,
                            new_relative_path: new_relative,
                            new_remote_uri: new_entry.uri.clone(),
                        }],
                        ..Default::default()
                    };
                    let worker_config = self.snapshot_worker_config().await;
                    let conflict_resolver = self.conflict.read().await.clone();
                    self.worker_pool.submit_background(
                        plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                    ).await;
                } else {
                    // 旧文件不存在本地，直接下载到新位置
                    let remote_entry = self.get_remote_entry_or_fallback(new_entry).await;
                    let plan = SyncPlan {
                        downloads: vec![SyncAction {
                            relative_path: new_relative,
                            local_entry: None,
                            remote_entry: Some(remote_entry),
                            db_mapping: None,
                        }],
                        ..Default::default()
                    };
                    let worker_config = self.snapshot_worker_config().await;
                    let conflict_resolver = self.conflict.read().await.clone();
                    self.worker_pool.submit_background(
                        plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                    ).await;
                }
            }
            RemoteFileEvent::Moved { old_uri, new_entry } => {
                let old_relative = crate::diff::remote_relative_path(
                    remote_root,
                    old_uri,
                    &new_entry.name,
                    false,
                );
                let new_relative = crate::diff::remote_relative_path(
                    remote_root,
                    &new_entry.path,
                    &new_entry.name,
                    new_entry.is_dir,
                );
                tracing::info!("[远程事件] 移动: {} -> {}", old_relative, new_relative);

                // 抑制本地 debouncer 自触发事件
                let now = std::time::Instant::now();
                self.suppress_paths.insert(old_relative.clone(), now);
                self.suppress_paths.insert(new_relative.clone(), now);

                let old_local_path = local_root.join(&old_relative);

                if old_local_path.exists() {
                    let plan = SyncPlan {
                        move_local: vec![LocalRenameAction {
                            old_relative_path: old_relative,
                            new_relative_path: new_relative,
                            new_remote_uri: new_entry.uri.clone(),
                        }],
                        ..Default::default()
                    };
                    let worker_config = self.snapshot_worker_config().await;
                    let conflict_resolver = self.conflict.read().await.clone();
                    self.worker_pool.submit_background(
                        plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                    ).await;
                } else {
                    let remote_entry = self.get_remote_entry_or_fallback(new_entry).await;
                    let plan = SyncPlan {
                        downloads: vec![SyncAction {
                            relative_path: new_relative,
                            local_entry: None,
                            remote_entry: Some(remote_entry),
                            db_mapping: None,
                        }],
                        ..Default::default()
                    };
                    let worker_config = self.snapshot_worker_config().await;
                    let conflict_resolver = self.conflict.read().await.clone();
                    self.worker_pool.submit_background(
                        plan, worker_config, WorkerTrigger::Continuous, conflict_resolver,
                    ).await;
                }
            }
        }
    }

    /// 获取远程文件详情，失败则使用 SSE 数据回退
    async fn get_remote_entry_or_fallback(&self, entry: &RemoteFileEntry) -> RemoteFileEntry {
        if entry.size == 0 && !entry.is_dir {
            match self.api.get_file_info(&entry.uri).await {
                Ok(info) => info,
                Err(_) => entry.clone(),
            }
        } else {
            entry.clone()
        }
    }

    pub async fn stop(&self) -> Result<()> {
        self.shutdown_token.lock().unwrap().cancel();
        Ok(())
    }

    /// 重置同步：停止任务 → 清空 DB → 清空本地目录 → 回到初始状态
    pub async fn reset_sync(&self) -> Result<()> {
        tracing::info!("开始重置同步...");

        // 1. 停止同步
        self.shutdown_token.lock().unwrap().cancel();

        // 2. 终止所有活跃 Worker
        self.worker_pool.abort_all_workers().await;

        // 3. 清空 DB 业务数据
        self.db.reset_sync_data().await?;
        tracing::info!("同步数据库已清空");

        // 4. 清空本地同步目录（保留目录本身，只删内容）
        let local_root = self.config.read().await.local_root.clone();
        if local_root.exists() {
            let entries = std::fs::read_dir(&local_root)
                .map_err(|_| crate::errors::SyncError::DiskFull { needed: 0, available: 0 })?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
            tracing::info!("本地同步目录已清空: {}", local_root.display());
        }

        // 5. 清空内存缓存
        self.ensured_dirs.clear();
        self.suppress_paths.clear();

        // 6. 重置状态
        *self.state.write().await = SyncState::Idle;

        tracing::info!("同步重置完成，已回到初始状态");
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

        *self.conflict.write().await = ConflictResolver::new(new_config.conflict_strategy.clone());

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

    pub async fn sync_album(&self, album_paths: Vec<String>, remote_dcim_uri: &str) -> Result<()> {
        let synced = self.db.get_album_sync_records().await?;
        let new_photos: Vec<_> = album_paths.iter().filter(|p| !synced.contains_key(*p)).collect();
        let total = new_photos.len();
        if total == 0 { return Ok(()); }

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
                            match crate::uploader::upload_file_chunked(&self.api, local_path, &session).await {
                                Ok(_) => {
                                    let remote_uri = format!("{}/{}", remote_dcim_uri, file_name);
                                    let hash = crate::utils::quick_hash(local_path, file_size).await.unwrap_or_default();
                                    if let Err(e) = self.db.add_album_sync_record(photo_path, &remote_uri, &hash).await {
                                        tracing::warn!("记录同步状态失败: {}", e);
                                    }
                                    tracing::info!("照片上传完成 ({}/{}): {}", i + 1, total, file_name);
                                }
                                Err(e) => tracing::error!("上传照片失败 {}: {}", file_name, e),
                            }
                        }
                        Err(e) => tracing::error!("创建上传会话失败 {}: {}", file_name, e),
                    }
                }
                Err(e) => tracing::warn!("无法读取照片元数据 {}: {}", photo_path, e),
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
        self.shutdown_token.lock().unwrap().cancel();
        Ok(())
    }

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

fn event_type_name(event: &RemoteFileEvent) -> &'static str {
    match event {
        RemoteFileEvent::Created(_) => "create",
        RemoteFileEvent::Modified(_) => "modify",
        RemoteFileEvent::Deleted { .. } => "delete",
        RemoteFileEvent::Renamed { .. } => "rename",
        RemoteFileEvent::Moved { .. } => "move",
    }
}

/// 从两个绝对路径生成相对于 local_root 的相对路径对
fn rel_pair(local_root: &std::path::Path, old_path: &std::path::Path, new_path: &std::path::Path) -> Option<(String, String)> {
    let old_rel = old_path.strip_prefix(local_root).ok()?
        .to_string_lossy().to_string();
    let new_rel = new_path.strip_prefix(local_root).ok()?
        .to_string_lossy().to_string();
    Some((crate::utils::normalize_path(&old_rel), crate::utils::normalize_path(&new_rel)))
}

fn find_top_level_dirs(dirs: &[String]) -> Vec<String> {
    if dirs.is_empty() { return Vec::new(); }

    let mut sorted: Vec<&String> = dirs.iter().collect();
    sorted.sort();

    let mut top_level = Vec::new();
    for dir in &sorted {
        let dominated = top_level.iter().any(|parent: &String| {
            dir.starts_with(parent.as_str())
                && dir.as_bytes().get(parent.len()) == Some(&b'/')
        });
        if !dominated {
            top_level.retain(|existing: &String| {
                !existing.starts_with(dir.as_str())
                    || existing.as_bytes().get(dir.len()) != Some(&b'/')
            });
            top_level.push((*dir).clone());
        }
    }

    top_level
}

/// 检测文件是否稳定（不在被写入中）
///
/// 策略：尝试以独占读方式打开文件，如果失败说明文件正被其他进程写入。
/// 同时对比两次采样的文件大小，如果大小变化则说明还在写入中。
async fn is_file_stable(path: &Path) -> bool {
    // 第一次采样大小
    let size1 = match tokio::fs::metadata(path).await {
        Ok(m) if !m.is_dir() => m.len(),
        _ => return false,
    };

    // 尝试独占打开：Windows 上如果文件被占用会失败
    // 使用 std::fs 以便跨平台兼容
    let path_display = path.display().to_string();
    let can_open = tokio::task::spawn_blocking(move || {
        match std::fs::File::open(&path_display) {
            Ok(_) => true,
            Err(e) => {
                // 文件被占用（Windows SharingViolation 等）→ 不稳定
                // 其他错误（文件不存在等）→ 也不稳定
                tracing::trace!("文件稳定性检测打开失败: {}", e);
                false
            }
        }
    })
    .await
    .unwrap_or(false);

    if !can_open {
        return false;
    }

    // 短暂等待后再次采样大小
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let size2 = match tokio::fs::metadata(path).await {
        Ok(m) if !m.is_dir() => m.len(),
        _ => return false,
    };

    size1 == size2 && size1 > 0
}
