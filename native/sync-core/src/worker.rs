use crate::api_client::ApiClient;
use crate::conflict_resolver::ConflictResolver;
use crate::errors::{Result, SyncError};
use crate::file_lock::FileLockRegistry;
use crate::models::*;
use crate::sync_db::SyncDb;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// Worker — 最小调度单元，拥有独立 UUID 和配置快照
pub struct Worker {
    pub task_id: String,
    trigger: WorkerTrigger,
    config: WorkerConfig,
    plan: SyncPlan,
    db: Arc<SyncDb>,
    api: Arc<ApiClient>,
    file_locks: Arc<FileLockRegistry>,
    ensured_dirs: Arc<DashMap<String, ()>>,
    conflict_resolver: ConflictResolver,
    event_sink: Arc<crate::event_sink::EventSink>,
    shutdown_token: CancellationToken,
}

impl Worker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_id: String,
        trigger: WorkerTrigger,
        config: WorkerConfig,
        plan: SyncPlan,
        db: Arc<SyncDb>,
        api: Arc<ApiClient>,
        file_locks: Arc<FileLockRegistry>,
        ensured_dirs: Arc<DashMap<String, ()>>,
        conflict_resolver: ConflictResolver,
        event_sink: Arc<crate::event_sink::EventSink>,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            task_id,
            trigger,
            config,
            plan,
            db,
            api,
            file_locks,
            ensured_dirs,
            conflict_resolver,
            event_sink,
            shutdown_token,
        }
    }

    /// 执行 Worker 任务
    pub async fn run(mut self) -> Result<SyncSummary> {
        let start = Instant::now();
        let tid = self.task_id.clone();
        let trigger_str = self.trigger.as_str();

        tracing::info!(
            "[{}] Worker启动: trigger={}, 上传={}, 下载={}, 删本地={}, 删远程={}, 冲突={}",
            tid, trigger_str,
            self.plan.uploads.len(),
            self.plan.downloads.len(),
            self.plan.delete_local.len(),
            self.plan.delete_remote.len(),
            self.plan.conflicts.len(),
        );

        // 更新 DB: status = running
        let _ = self.db.update_sync_task_status(&tid, &WorkerStatus::Running).await;

        // 创建本地传输并发信号量
        let transfer_semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_transfers.max(1)));

        let mut summary = SyncSummary::default();
        let root_id = self.config.sync_root_id.clone();

        // 1. 创建远程目录结构
        for dir_path in &self.plan.mkdirs_remote {
            if self.shutdown_token.is_cancelled() { break; }
            match crate::uploader::ensure_remote_dirs(
                &tid, &self.config.remote_root, dir_path, &self.api, &self.ensured_dirs,
            ).await {
                Ok(_) => tracing::debug!("[{}] 创建远程目录: {}", tid, dir_path),
                Err(e) => tracing::warn!("[{}] 创建远程目录失败 {}: {}", tid, dir_path, e),
            }
        }

        // 2. 创建本地目录结构
        for dir_path in &self.plan.mkdirs_local {
            if self.shutdown_token.is_cancelled() { break; }
            let local_path = self.config.local_root.join(dir_path);
            if let Err(e) = tokio::fs::create_dir_all(&local_path).await {
                tracing::warn!("[{}] 创建本地目录失败 {}: {}", tid, dir_path, e);
            }
        }

        // 2.5 递归扫描 scan_dirs 中的目录，将文件加入 uploads
        if !self.plan.scan_dirs.is_empty() {
            tracing::info!("[{}] 开始递归扫描 {} 个目录", tid, self.plan.scan_dirs.len());
            let scanner = crate::fs_scanner::FsScanner::new();
            for dir_rel in &self.plan.scan_dirs {
                if self.shutdown_token.is_cancelled() { break; }
                let dir_path = self.config.local_root.join(dir_rel);
                if !dir_path.is_dir() {
                    tracing::warn!("[{}] 目录不存在，跳过: {}", tid, dir_path.display());
                    continue;
                }
                match scanner.scan(&dir_path, 50, false).await {
                    Ok(entries) => {
                        for entry in entries {
                            // 跳过 size=0 的普通文件
                            if !entry.is_dir && entry.size == 0 {
                                tracing::debug!("[{}] 跳过空文件: {}", tid, entry.relative_path.display());
                                continue;
                            }
                            // 重新计算相对于 local_root 的路径
                            let full_relative = if dir_rel.is_empty() {
                                entry.relative_path.to_string_lossy().to_string()
                            } else {
                                format!("{}/{}", dir_rel, entry.relative_path.to_string_lossy())
                            };
                            let full_relative = crate::utils::normalize_path(&full_relative);

                            let root_id_c = root_id.clone();
                            let db_mapping = self.db.get_file_mapping(&root_id_c, &full_relative).await.ok().flatten();
                            if let Some(ref mapping) = db_mapping {
                                if mapping.local_hash.as_deref() == Some(&entry.quick_hash) {
                                    continue;
                                }
                            }

                            if entry.is_dir {
                                self.plan.mkdirs_remote.push(full_relative.clone());
                            } else {
                                let mut local_entry = entry.clone();
                                local_entry.relative_path = std::path::PathBuf::from(&full_relative);
                                self.plan.uploads.push(SyncAction {
                                    relative_path: full_relative,
                                    local_entry: Some(local_entry),
                                    remote_entry: None,
                                    db_mapping,
                                });
                            }
                        }
                        tracing::info!("[{}] 目录扫描完成: {}", tid, dir_rel);
                    }
                    Err(e) => {
                        tracing::error!("[{}] 扫描目录失败 {}: {}", tid, dir_rel, e);
                    }
                }
            }

            // 为扫描目录新增的 uploads 和 mkdirs_remote 创建 task_item 记录
            let now = chrono::Utc::now().to_rfc3339();
            for action in &self.plan.uploads {
                // 仅创建从 scan_dirs 扫描出的条目的 task_item（避免重复创建）
                let is_from_scan = self.plan.scan_dirs.iter().any(|dir| {
                    action.relative_path.starts_with(dir.as_str())
                        && action.relative_path.as_bytes().get(dir.len()) == Some(&b'/')
                });
                if is_from_scan {
                    let item = SyncTaskItem {
                        id: 0,
                        task_id: tid.clone(),
                        relative_path: action.relative_path.clone(),
                        action_type: TaskActionType::Upload,
                        status: TaskItemStatus::Pending,
                        file_size: action.local_entry.as_ref().map(|l| l.size).unwrap_or(0),
                        error_message: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };
                    let _ = self.db.create_sync_task_item(&item).await;
                }
            }
            for dir_path in &self.plan.mkdirs_remote {
                let is_from_scan = self.plan.scan_dirs.iter().any(|dir| {
                    dir_path.starts_with(dir.as_str())
                        && dir_path.as_bytes().get(dir.len()) == Some(&b'/')
                });
                if is_from_scan {
                    let item = SyncTaskItem {
                        id: 0,
                        task_id: tid.clone(),
                        relative_path: dir_path.clone(),
                        action_type: TaskActionType::MkdirRemote,
                        status: TaskItemStatus::Pending,
                        file_size: 0,
                        error_message: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };
                    let _ = self.db.create_sync_task_item(&item).await;
                }
            }

            // 更新 DB 任务的总数
            let new_total = self.plan.uploads.len() as u32
                + self.plan.downloads.len() as u32
                + self.plan.delete_local.len() as u32
                + self.plan.delete_remote.len() as u32
                + self.plan.conflicts.len() as u32;
            let _ = self.db.update_sync_task_total_count(&tid, new_total).await;
        }

        // 3. 处理冲突（串行）
        for conflict in &self.plan.conflicts {
            if self.shutdown_token.is_cancelled() { break; }
            let local_mtime = conflict.local_entry.as_ref().map(|l| l.mtime_ms).unwrap_or(0);
            let remote_mtime = conflict.remote_entry.as_ref().map(|r| r.mtime_ms).unwrap_or(0);
            let local_size = conflict.local_entry.as_ref().map(|l| l.size).unwrap_or(0);
            let remote_size = conflict.remote_entry.as_ref().map(|r| r.size).unwrap_or(0);
            let local_name = conflict.local_entry.as_ref()
                .map(|l| l.relative_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default())
                .unwrap_or_default();

            let resolution = self.conflict_resolver.resolve(
                conflict.conflict_type.clone(),
                local_mtime,
                remote_mtime,
                local_size,
                remote_size,
                &local_name,
            );

            tracing::info!("[{}] 冲突解决: {} → {:?}", tid, conflict.relative_path, resolution);

            match resolution {
                ConflictResolution::UploadLocal => {
                    if let Some(ref local) = conflict.local_entry {
                        let action = SyncAction {
                            relative_path: conflict.relative_path.clone(),
                            local_entry: Some(local.clone()),
                            remote_entry: conflict.remote_entry.clone(),
                            db_mapping: conflict.db_mapping.clone(),
                        };
                        match crate::uploader::upload_file(
                            &tid, &action, &self.config, &self.api, &self.db,
                            &self.file_locks, &self.ensured_dirs, &transfer_semaphore, &root_id,
                        ).await {
                            Ok(_) => summary.uploaded += 1,
                            Err(e) => {
                                tracing::error!("[{}] 冲突上传失败: {}: {}", tid, conflict.relative_path, e);
                                summary.conflicts += 1;
                            }
                        }
                    }
                }
                ConflictResolution::DownloadRemote => {
                    if let Some(ref remote) = conflict.remote_entry {
                        let action = SyncAction {
                            relative_path: conflict.relative_path.clone(),
                            local_entry: conflict.local_entry.clone(),
                            remote_entry: Some(remote.clone()),
                            db_mapping: conflict.db_mapping.clone(),
                        };
                        match crate::downloader::download_file(
                            &tid, &action, &self.config, &self.api, &self.db,
                            &self.file_locks, &transfer_semaphore, &root_id,
                        ).await {
                            Ok(_) => summary.downloaded += 1,
                            Err(e) => {
                                tracing::error!("[{}] 冲突下载失败: {}: {}", tid, conflict.relative_path, e);
                                summary.conflicts += 1;
                            }
                        }
                    }
                }
                ConflictResolution::RenameLocal { ref new_name } => {
                    if let Some(ref local) = conflict.local_entry {
                        let old_path = self.config.local_root.join(&local.relative_path);
                        let mut new_rel_path = local.relative_path.clone();
                        new_rel_path.pop();
                        new_rel_path.push(new_name);
                        let new_rel = crate::utils::normalize_path(&new_rel_path.to_string_lossy());
                        let new_path = self.config.local_root.join(&new_rel_path);
                        let mut rename_retries = 0u32;
                        let renamed = loop {
                            match tokio::fs::rename(&old_path, &new_path).await {
                                Ok(_) => break true,
                                Err(e) if e.raw_os_error() == Some(5) && rename_retries < 10 => {
                                    rename_retries += 1;
                                    let delay = rename_retries as u64 * 1000;
                                    tracing::warn!("[{}] 重命名文件被占用，{}ms后重试 ({})", tid, delay, rename_retries);
                                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                                }
                                Err(e) => {
                                    tracing::warn!("[{}] 重命名冲突文件失败: {}", tid, e);
                                    break false;
                                }
                            }
                        };
                        if renamed {
                            tracing::info!("[{}] 冲突文件已重命名保留在本地: {}", tid, new_rel);
                            if let Some(ref remote) = conflict.remote_entry {
                                let action = SyncAction {
                                    relative_path: conflict.relative_path.clone(),
                                    local_entry: None,
                                    remote_entry: Some(remote.clone()),
                                    db_mapping: None,
                                };
                                if let Err(e) = crate::downloader::download_file(
                                    &tid, &action, &self.config, &self.api, &self.db,
                                    &self.file_locks, &transfer_semaphore, &root_id,
                                ).await {
                                    tracing::warn!("[{}] 下载远程冲突版本失败: {}", tid, e);
                                } else {
                                    summary.downloaded += 1;
                                }
                            }
                        }
                    }
                }
                ConflictResolution::DeleteLocal => {
                    if let Some(ref local) = conflict.local_entry {
                        let local_path = self.config.local_root.join(&local.relative_path);
                        let _ = tokio::fs::remove_file(&local_path).await;
                        summary.deleted_local += 1;
                        tracing::info!("[{}] 删除本地(冲突解决): {}", tid, conflict.relative_path);
                    }
                }
                ConflictResolution::DeleteRemote => {
                    if let Some(ref remote) = conflict.remote_entry {
                        let _ = self.api.delete_files(&[&remote.uri]).await;
                        summary.deleted_remote += 1;
                        tracing::info!("[{}] 删除远程(冲突解决): {}", tid, conflict.relative_path);
                    }
                }
                ConflictResolution::MarkManual => {
                    summary.conflicts += 1;
                    tracing::warn!("[{}] 手动解决冲突: {}", tid, conflict.relative_path);
                }
            }
        }

        // 4. 并发上传
        let mut upload_handles = Vec::new();
        for action in &self.plan.uploads {
            if self.shutdown_token.is_cancelled() { break; }
            let action = action.clone();
            let tid_c = tid.clone();
            let config = self.config.clone();
            let api = self.api.clone();
            let db = self.db.clone();
            let file_locks = self.file_locks.clone();
            let ensured_dirs = self.ensured_dirs.clone();
            let sem = transfer_semaphore.clone();
            let root_id_c = root_id.clone();

            let handle = tokio::spawn(async move {
                crate::uploader::upload_file(
                    &tid_c, &action, &config, &api, &db,
                    &file_locks, &ensured_dirs, &sem, &root_id_c,
                ).await
            });
            upload_handles.push(handle);
        }

        for handle in upload_handles {
            match handle.await {
                Ok(Ok(_)) => {
                    summary.uploaded += 1;
                    let _ = self.db.increment_task_completed(&tid).await;
                }
                Ok(Err(e)) => {
                    tracing::error!("[{}] 上传失败: {}", tid, e);
                    summary.conflicts += 1;
                    let _ = self.db.increment_task_failed(&tid).await;
                }
                Err(e) => {
                    tracing::error!("[{}] 上传任务异常: {}", tid, e);
                    summary.conflicts += 1;
                    let _ = self.db.increment_task_failed(&tid).await;
                }
            }
        }

        // 5. 并发下载
        let mut download_handles = Vec::new();
        for action in &self.plan.downloads {
            if self.shutdown_token.is_cancelled() { break; }
            let action = action.clone();
            let tid_c = tid.clone();
            let config = self.config.clone();
            let api = self.api.clone();
            let db = self.db.clone();
            let file_locks = self.file_locks.clone();
            let sem = transfer_semaphore.clone();
            let root_id_c = root_id.clone();

            let handle = tokio::spawn(async move {
                crate::downloader::download_file(
                    &tid_c, &action, &config, &api, &db,
                    &file_locks, &sem, &root_id_c,
                ).await
            });
            download_handles.push(handle);
        }

        for handle in download_handles {
            match handle.await {
                Ok(Ok(_)) => {
                    summary.downloaded += 1;
                    let _ = self.db.increment_task_completed(&tid).await;
                }
                Ok(Err(e)) => {
                    tracing::error!("[{}] 下载失败: {}", tid, e);
                    summary.conflicts += 1;
                    let _ = self.db.increment_task_failed(&tid).await;
                }
                Err(e) => {
                    tracing::error!("[{}] 下载任务异常: {}", tid, e);
                    summary.conflicts += 1;
                    let _ = self.db.increment_task_failed(&tid).await;
                }
            }
        }

        // 6. 删除本地文件
        for action in &self.plan.delete_local {
            if self.shutdown_token.is_cancelled() { break; }
            if let Some(ref local) = action.local_entry {
                let local_path = self.config.local_root.join(&local.relative_path);
                match tokio::fs::remove_file(&local_path).await {
                    Ok(_) => {
                        summary.deleted_local += 1;
                        tracing::info!("[{}] 删除本地: {}", tid, action.relative_path);
                        let _ = self.db.delete_file_mapping(&root_id, &action.relative_path).await;
                    }
                    Err(e) => tracing::warn!("[{}] 删除本地文件失败 {}: {}", tid, action.relative_path, e),
                }
            }
        }

        // 7. 删除远程文件
        let remote_uris: Vec<&str> = self.plan.delete_remote.iter()
            .filter_map(|a| a.remote_entry.as_ref().map(|r| r.uri.as_str()))
            .collect();
        if !remote_uris.is_empty() {
            match self.api.delete_files(&remote_uris).await {
                Ok(_) => {
                    summary.deleted_remote += remote_uris.len() as u32;
                    for uri in &remote_uris {
                        tracing::info!("[{}] 删除远程: {}", tid, uri);
                    }
                }
                Err(e) => tracing::error!("[{}] 批量删除远程文件失败: {}", tid, e),
            }
            for action in &self.plan.delete_remote {
                let _ = self.db.delete_file_mapping(&root_id, &action.relative_path).await;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let final_status = if self.shutdown_token.is_cancelled() {
            WorkerStatus::Cancelled
        } else if summary.conflicts > 0 && summary.uploaded + summary.downloaded == 0 {
            WorkerStatus::Failed
        } else {
            WorkerStatus::Completed
        };

        let _ = self.db.finish_sync_task(
            &tid, &final_status, summary.uploaded + summary.downloaded, summary.conflicts,
        ).await;

        tracing::info!(
            "[{}] Worker完成: 上传={}, 下载={}, 失败={}, 跳过={}, 删本地={}, 删远程={}, 耗时={}ms",
            tid, summary.uploaded, summary.downloaded, summary.conflicts, summary.skipped,
            summary.deleted_local, summary.deleted_remote, duration_ms,
        );

        summary.duration_ms = duration_ms;

        // 推送事件到 Dart
        let _ = self.event_sink.emit(crate::api::ffi_types::SyncEventFfi::WorkerCompleted {
            task_id: tid.clone(),
            uploaded: summary.uploaded,
            downloaded: summary.downloaded,
            failed: summary.conflicts,
            duration_ms,
        }).await;

        Ok(summary)
    }
}

/// WorkerPool — 全局 Worker 并发控制
pub struct WorkerPool {
    worker_semaphore: Arc<Semaphore>,
    active_workers: DashMap<String, tokio::task::JoinHandle<()>>,
    db: Arc<SyncDb>,
    api: Arc<ApiClient>,
    file_locks: Arc<FileLockRegistry>,
    ensured_dirs: Arc<DashMap<String, ()>>,
    event_sink: Arc<crate::event_sink::EventSink>,
    shutdown_token: CancellationToken,
}

impl WorkerPool {
    pub fn new(
        db: Arc<SyncDb>,
        api: Arc<ApiClient>,
        file_locks: Arc<FileLockRegistry>,
        ensured_dirs: Arc<DashMap<String, ()>>,
        event_sink: Arc<crate::event_sink::EventSink>,
        shutdown_token: CancellationToken,
    ) -> Self {
        let cpu_count = num_cpus();
        let max_workers = cpu_count.clamp(1, 32);
        tracing::info!("WorkerPool 初始化: 最大并发 Worker 数={}", max_workers);

        Self {
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: DashMap::new(),
            db,
            api,
            file_locks,
            ensured_dirs,
            event_sink,
            shutdown_token,
        }
    }

    /// 提交 Worker（阻塞等待结果）
    pub async fn submit(
        &self,
        plan: SyncPlan,
        config: WorkerConfig,
        trigger: WorkerTrigger,
        conflict_resolver: ConflictResolver,
    ) -> Result<SyncSummary> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // 创建 DB 任务记录
        let total_count = plan.uploads.len() as u32
            + plan.downloads.len() as u32
            + plan.delete_local.len() as u32
            + plan.delete_remote.len() as u32
            + plan.conflicts.len() as u32;

        let task = SyncTask {
            id: task_id.clone(),
            trigger: trigger.clone(),
            total_count,
            completed_count: 0,
            failed_count: 0,
            status: WorkerStatus::Pending,
            created_at: now.clone(),
            updated_at: now,
            finished_at: None,
        };
        self.db.create_sync_task(&task).await?;

        // 创建 task_item 记录
        let now_for_items = chrono::Utc::now().to_rfc3339();
        self.create_task_items(&task_id, &plan, &now_for_items).await?;

        // 等待 Worker 信号量
        let _permit = self.worker_semaphore.acquire().await
            .map_err(|e| SyncError::Internal(format!("获取 Worker 信号量失败: {}", e)))?;

        let worker = Worker::new(
            task_id.clone(),
            trigger,
            config,
            plan,
            self.db.clone(),
            self.api.clone(),
            self.file_locks.clone(),
            self.ensured_dirs.clone(),
            conflict_resolver,
            self.event_sink.clone(),
            self.shutdown_token.clone(),
        );

        // 推送 WorkerStarted 事件
        let _ = self.event_sink.emit(crate::api::ffi_types::SyncEventFfi::WorkerStarted {
            task_id: task_id.clone(),
            trigger: task.trigger.as_str().to_string(),
            upload_count: task.total_count,
            download_count: 0,
        }).await;

        let result = worker.run().await;
        self.active_workers.remove(&task_id);
        result
    }

    /// 提交 Worker（火力全忘，后台运行）
    pub async fn submit_background(
        &self,
        plan: SyncPlan,
        config: WorkerConfig,
        trigger: WorkerTrigger,
        conflict_resolver: ConflictResolver,
    ) -> String {
        let task_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let total_count = plan.uploads.len() as u32
            + plan.downloads.len() as u32
            + plan.delete_local.len() as u32
            + plan.delete_remote.len() as u32
            + plan.conflicts.len() as u32;

        let task = SyncTask {
            id: task_id.clone(),
            trigger: trigger.clone(),
            total_count,
            completed_count: 0,
            failed_count: 0,
            status: WorkerStatus::Pending,
            created_at: now.clone(),
            updated_at: now,
            finished_at: None,
        };
        if let Err(e) = self.db.create_sync_task(&task).await {
            tracing::error!("创建同步任务记录失败: {}", e);
        }

        let now_for_items = chrono::Utc::now().to_rfc3339();
        if let Err(e) = self.create_task_items(&task_id, &plan, &now_for_items).await {
            tracing::warn!("创建任务项记录失败: {}", e);
        }

        let tid = task_id.clone();
        let trigger_str = trigger.as_str().to_string();
        let upload_count = plan.uploads.len() as u32;
        let download_count = plan.downloads.len() as u32;

        // 推送 WorkerStarted 事件
        let _ = self.event_sink.emit(crate::api::ffi_types::SyncEventFfi::WorkerStarted {
            task_id: tid.clone(),
            trigger: trigger_str,
            upload_count,
            download_count,
        }).await;

        let sem = self.worker_semaphore.clone();
        let db = self.db.clone();
        let api = self.api.clone();
        let file_locks = self.file_locks.clone();
        let ensured_dirs = self.ensured_dirs.clone();
        let event_sink = self.event_sink.clone();
        let shutdown_token = self.shutdown_token.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let worker = Worker::new(
                task_id.clone(),
                trigger,
                config,
                plan,
                db,
                api,
                file_locks,
                ensured_dirs,
                conflict_resolver,
                event_sink,
                shutdown_token,
            );
            if let Err(e) = worker.run().await {
                tracing::error!("[{}] Worker后台执行失败: {}", task_id, e);
            }
        });

        self.active_workers.insert(tid.clone(), handle);
        tid
    }

    /// 当前活跃 Worker 数
    pub fn active_worker_count(&self) -> usize {
        self.active_workers.len()
    }

    /// 创建 task_item 记录
    async fn create_task_items(&self, task_id: &str, plan: &SyncPlan, now: &str) -> Result<()> {
        for action in &plan.uploads {
            let item = SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::Upload,
                status: TaskItemStatus::Pending,
                file_size: action.local_entry.as_ref().map(|l| l.size).unwrap_or(0),
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };
            self.db.create_sync_task_item(&item).await?;
        }
        for action in &plan.downloads {
            let item = SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::Download,
                status: TaskItemStatus::Pending,
                file_size: action.remote_entry.as_ref().map(|r| r.size).unwrap_or(0),
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };
            self.db.create_sync_task_item(&item).await?;
        }
        for action in &plan.delete_local {
            let item = SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::DeleteLocal,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };
            self.db.create_sync_task_item(&item).await?;
        }
        for action in &plan.delete_remote {
            let item = SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::DeleteRemote,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };
            self.db.create_sync_task_item(&item).await?;
        }
        for conflict in &plan.conflicts {
            let item = SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: conflict.relative_path.clone(),
                action_type: TaskActionType::ConflictResolve,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };
            self.db.create_sync_task_item(&item).await?;
        }
        Ok(())
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
