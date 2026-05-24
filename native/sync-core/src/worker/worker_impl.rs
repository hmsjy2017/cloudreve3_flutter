use crate::api_client::ApiClient;
use crate::conflict_resolver::ConflictResolver;
use crate::errors::Result;
use crate::file_lock::FileLockRegistry;
use crate::models::*;
use crate::sync_db::SyncDb;
use dashmap::DashMap;
#[cfg(feature = "windows-cfapi")]
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// 占位符创建接口（由 platform::wcf 模块实现）
#[cfg(feature = "windows-cfapi")]
#[async_trait::async_trait]
pub trait PlaceholderCreator: Send + Sync {
    async fn create_placeholder_file(
        &self,
        base_dir: &Path,
        file_name: String,
        file_size: u64,
        file_identity: &[u8],
    ) -> Result<()>;
}

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
    #[cfg(feature = "windows-cfapi")]
    platform_adapter: Option<Arc<dyn PlaceholderCreator>>,
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
        #[cfg(feature = "windows-cfapi")] platform_adapter: Option<Arc<dyn PlaceholderCreator>>,
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
            #[cfg(feature = "windows-cfapi")]
            platform_adapter,
        }
    }

    /// 执行 Worker 任务（编排器，调用各步骤方法）
    pub async fn run(mut self) -> Result<SyncSummary> {
        let start = Instant::now();
        let tid = self.task_id.clone();
        let trigger_str = self.trigger.as_str();

        tracing::info!(
            "[{}] Worker启动: trigger={}, 上传={}, 下载={}, 删本地={}, 删远程={}, 重命名={}, 移动={}, 冲突={}",
            tid, trigger_str,
            self.plan.uploads.len(),
            self.plan.downloads.len(),
            self.plan.delete_local.len(),
            self.plan.delete_remote.len(),
            self.plan.rename_remote.len(),
            self.plan.move_remote.len(),
            self.plan.conflicts.len(),
        );

        let _ = self
            .db
            .update_sync_task_status(&tid, &WorkerStatus::Running)
            .await;

        // 全局传输并发信号量，冲突/上传/下载共享
        let transfer_semaphore =
            Arc::new(Semaphore::new(self.config.max_concurrent_transfers.max(1)));

        let mut summary = SyncSummary::default();

        self.step_create_remote_dirs().await;
        self.step_create_local_dirs().await;
        self.step_rename_remote(&mut summary).await;
        self.step_move_remote(&mut summary).await;
        self.step_scan_new_dirs(&mut summary).await;
        self.step_resolve_conflicts(&mut summary, &transfer_semaphore).await;
        self.step_execute_uploads(&mut summary, &transfer_semaphore).await;
        self.step_execute_downloads_or_placeholders(&mut summary, &transfer_semaphore).await;
        self.step_delete_local(&mut summary).await;
        self.step_rename_local(&mut summary).await;
        self.step_move_local(&mut summary).await;
        self.step_delete_remote(&mut summary).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let final_status = if self.shutdown_token.is_cancelled() {
            WorkerStatus::Cancelled
        } else if summary.failed > 0
            && summary.uploaded + summary.downloaded + summary.renamed + summary.moved == 0
        {
            WorkerStatus::Failed
        } else {
            WorkerStatus::Completed
        };

        let completed_count = summary.uploaded
            + summary.downloaded
            + summary.renamed
            + summary.moved
            + summary.deleted_local
            + summary.deleted_remote;
        let _ = self
            .db
            .finish_sync_task(&tid, &final_status, completed_count, summary.failed)
            .await;

        tracing::info!(
            "[{}] Worker完成: 上传={}, 下载={}, 失败={}, 跳过={}, 重命名={}, 移动={}, 删本地={}, 删远程={}, 冲突={}, 耗时={}ms",
            tid, summary.uploaded, summary.downloaded, summary.failed, summary.skipped,
            summary.renamed, summary.moved, summary.deleted_local, summary.deleted_remote,
            summary.conflicts, duration_ms,
        );

        summary.duration_ms = duration_ms;

        let _ = self
            .event_sink
            .emit(crate::api::ffi_types::SyncEventFfi::WorkerCompleted {
                task_id: tid.clone(),
                uploaded: summary.uploaded,
                downloaded: summary.downloaded,
                renamed: summary.renamed,
                moved: summary.moved,
                failed: summary.failed,
                duration_ms,
            })
            .await;

        Ok(summary)
    }

    // ===== 步骤方法 =====

    /// 1. 创建远程目录结构（UploadOnly / Full / MirrorWcf）
    async fn step_create_remote_dirs(&self) {
        if matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
            return;
        }
        let tid = &self.task_id;
        for dir_path in &self.plan.mkdirs_remote {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            match crate::uploader::ensure_remote_dirs(
                tid,
                &self.config.remote_root,
                dir_path,
                &self.api,
                &self.ensured_dirs,
            )
            .await
            {
                Ok(_) => tracing::debug!("[{}] 创建远程目录: {}", tid, dir_path),
                Err(e) => tracing::warn!("[{}] 创建远程目录失败 {}: {}", tid, dir_path, e),
            }
        }
    }

    /// 2. 创建本地目录结构（DownloadOnly / Full / MirrorWcf）
    async fn step_create_local_dirs(&self) {
        if matches!(self.config.sync_mode, SyncMode::UploadOnly) {
            return;
        }
        let tid = &self.task_id;
        for dir_path in &self.plan.mkdirs_local {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let local_path = self.config.local_root.join(dir_path);
            if let Err(e) = tokio::fs::create_dir_all(&local_path).await {
                tracing::warn!("[{}] 创建本地目录失败 {}: {}", tid, dir_path, e);
            }
        }
    }

    /// 2.1 执行远程重命名（UploadOnly / Full / MirrorWcf）
    async fn step_rename_remote(&self, summary: &mut SyncSummary) {
        if matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
            return;
        }
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        for rename in &self.plan.rename_remote {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let rel_path = format!(
                "{} -> {}",
                rename.old_relative_path, rename.new_relative_path
            );
            match self
                .api
                .rename_file(&rename.remote_uri, &rename.new_name)
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        "[{}] 远程重命名: {} -> {}",
                        tid,
                        rename.old_relative_path,
                        rename.new_relative_path
                    );
                    summary.renamed += 1;
                    let _ = self.db.increment_task_completed(tid).await;
                    let new_remote_uri = {
                        let uri = &rename.remote_uri;
                        let last_slash = uri.trim_end_matches('/').rfind('/').unwrap_or(0);
                        format!("{}/{}", &uri[..last_slash], rename.new_name)
                    };
                    let _ = self
                        .db
                        .update_file_mapping_path(
                            root_id,
                            &rename.old_relative_path,
                            &rename.new_relative_path,
                            &new_remote_uri,
                        )
                        .await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "rename",
                            &TaskItemStatus::Completed,
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    tracing::error!(
                        "[{}] 远程重命名失败: {} -> {}: {}",
                        tid,
                        rename.old_relative_path,
                        rename.new_relative_path,
                        e
                    );
                    summary.failed += 1;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "rename",
                            &TaskItemStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        }
    }

    /// 2.2 执行远程移动（本地触发）
    async fn step_move_remote(&self, summary: &mut SyncSummary) {
        if matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
            return;
        }
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        for mov in &self.plan.move_remote {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let rel_path = format!("{} -> {}", mov.old_relative_path, mov.new_relative_path);
            match self
                .api
                .move_files(&[&mov.remote_uri], &mov.dst_remote_dir_uri, false)
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        "[{}] 远程移动: {} -> {}",
                        tid,
                        mov.old_relative_path,
                        mov.new_relative_path
                    );
                    summary.moved += 1;
                    let _ = self.db.increment_task_completed(tid).await;
                    let _ = self
                        .db
                        .update_file_mapping_path(
                            root_id,
                            &mov.old_relative_path,
                            &mov.new_relative_path,
                            &mov.dst_remote_dir_uri,
                        )
                        .await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "move",
                            &TaskItemStatus::Completed,
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    tracing::error!(
                        "[{}] 远程移动失败: {} -> {}: {}",
                        tid,
                        mov.old_relative_path,
                        mov.new_relative_path,
                        e
                    );
                    summary.failed += 1;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "move",
                            &TaskItemStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        }
    }

    /// 2.5 递归扫描 scan_dirs 中的目录，将文件加入 uploads
    async fn step_scan_new_dirs(&mut self, _summary: &mut SyncSummary) {
        if self.plan.scan_dirs.is_empty() {
            return;
        }
        let tid = &self.task_id;
        let root_id = self.config.sync_root_id.clone();

        tracing::info!(
            "[{}] 开始递归扫描 {} 个目录",
            tid,
            self.plan.scan_dirs.len()
        );
        let scanner = crate::fs_scanner::FsScanner::new();
        for dir_rel in &self.plan.scan_dirs {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let dir_path = self.config.local_root.join(dir_rel);
            if !dir_path.is_dir() {
                tracing::warn!("[{}] 目录不存在，跳过: {}", tid, dir_path.display());
                continue;
            }
            match scanner.scan(&dir_path, 50, false).await {
                Ok(entries) => {
                    for entry in entries {
                        if !entry.is_dir && entry.size == 0 {
                            tracing::debug!(
                                "[{}] 跳过空文件: {}",
                                tid,
                                entry.relative_path.display()
                            );
                            continue;
                        }
                        let full_relative = if dir_rel.is_empty() {
                            entry.relative_path.to_string_lossy().to_string()
                        } else {
                            format!("{}/{}", dir_rel, entry.relative_path.to_string_lossy())
                        };
                        let full_relative = crate::utils::normalize_path(&full_relative);

                        let root_id_c = root_id.clone();
                        let db_mapping = self
                            .db
                            .get_file_mapping(&root_id_c, &full_relative)
                            .await
                            .ok()
                            .flatten();
                        if let Some(ref mapping) = db_mapping {
                            if mapping.local_hash.as_deref() == Some(&entry.quick_hash) {
                                continue;
                            }
                        }

                        if entry.is_dir {
                            self.plan.mkdirs_remote.push(full_relative.clone());
                        } else {
                            let mut local_entry = entry.clone();
                            local_entry.relative_path =
                                std::path::PathBuf::from(&full_relative);
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

        let new_total = self.plan.uploads.len() as u32
            + self.plan.downloads.len() as u32
            + self.plan.delete_local.len() as u32
            + self.plan.delete_remote.len() as u32
            + self.plan.conflicts.len() as u32;
        let _ = self.db.update_sync_task_total_count(tid, new_total).await;
    }

    /// 3. 处理冲突（串行）
    async fn step_resolve_conflicts(&self, summary: &mut SyncSummary, transfer_semaphore: &Arc<Semaphore>) {
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        for conflict in &self.plan.conflicts {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let local_mtime = conflict
                .local_entry
                .as_ref()
                .map(|l| l.mtime_ms)
                .unwrap_or(0);
            let remote_mtime = conflict
                .remote_entry
                .as_ref()
                .map(|r| r.mtime_ms)
                .unwrap_or(0);
            let local_size = conflict.local_entry.as_ref().map(|l| l.size).unwrap_or(0);
            let remote_size = conflict.remote_entry.as_ref().map(|r| r.size).unwrap_or(0);
            let local_name = conflict
                .local_entry
                .as_ref()
                .map(|l| {
                    l.relative_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            let resolution = self.conflict_resolver.resolve(
                conflict.conflict_type.clone(),
                local_mtime,
                remote_mtime,
                local_size,
                remote_size,
                &local_name,
            );

            tracing::info!(
                "[{}] 冲突解决: {} → {:?}",
                tid,
                conflict.relative_path,
                resolution
            );

            let mut conflict_ok = false;
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
                            tid,
                            &action,
                            &self.config,
                            &self.api,
                            &self.db,
                            &self.file_locks,
                            &self.ensured_dirs,
                            transfer_semaphore,
                            root_id,
                        )
                        .await
                        {
                            Ok(_) => {
                                summary.uploaded += 1;
                                conflict_ok = true;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "[{}] 冲突上传失败: {}: {}",
                                    tid,
                                    conflict.relative_path,
                                    e
                                );
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
                            tid,
                            &action,
                            &self.config,
                            &self.api,
                            &self.db,
                            &self.file_locks,
                            transfer_semaphore,
                            root_id,
                        )
                        .await
                        {
                            Ok(_) => {
                                summary.downloaded += 1;
                                conflict_ok = true;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "[{}] 冲突下载失败: {}: {}",
                                    tid,
                                    conflict.relative_path,
                                    e
                                );
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
                                    tracing::warn!(
                                        "[{}] 重命名文件被占用，{}ms后重试 ({})",
                                        tid,
                                        delay,
                                        rename_retries
                                    );
                                    tokio::time::sleep(std::time::Duration::from_millis(delay))
                                        .await;
                                }
                                Err(e) => {
                                    tracing::warn!("[{}] 重命名冲突文件失败: {}", tid, e);
                                    break false;
                                }
                            }
                        };
                        if renamed {
                            conflict_ok = true;
                            tracing::info!("[{}] 冲突文件已重命名保留在本地: {}", tid, new_rel);
                            if let Some(ref remote) = conflict.remote_entry {
                                let action = SyncAction {
                                    relative_path: conflict.relative_path.clone(),
                                    local_entry: None,
                                    remote_entry: Some(remote.clone()),
                                    db_mapping: None,
                                };
                                if let Err(e) = crate::downloader::download_file(
                                    tid,
                                    &action,
                                    &self.config,
                                    &self.api,
                                    &self.db,
                                    &self.file_locks,
                                    transfer_semaphore,
                                    root_id,
                                )
                                .await
                                {
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
                        conflict_ok = true;
                        tracing::info!("[{}] 删除本地(冲突解决): {}", tid, conflict.relative_path);
                    }
                }
                ConflictResolution::DeleteRemote => {
                    if let Some(ref remote) = conflict.remote_entry {
                        let _ = self.api.delete_files(&[&remote.uri]).await;
                        summary.deleted_remote += 1;
                        conflict_ok = true;
                        tracing::info!("[{}] 删除远程(冲突解决): {}", tid, conflict.relative_path);
                    }
                }
                ConflictResolution::MarkManual => {
                    summary.conflicts += 1;
                    tracing::warn!("[{}] 手动解决冲突: {}", tid, conflict.relative_path);
                }
            }
            let item_status = if conflict_ok {
                TaskItemStatus::Completed
            } else {
                TaskItemStatus::Failed
            };
            if conflict_ok {
                let _ = self.db.increment_task_completed(tid).await;
            }
            let _ = self
                .db
                .update_task_item_status_by_path(
                    tid,
                    &conflict.relative_path,
                    "conflict_resolve",
                    &item_status,
                    None,
                )
                .await;
        }
    }

    /// 4. 并发上传（UploadOnly / Full / MirrorWcf）
    async fn step_execute_uploads(&self, summary: &mut SyncSummary, transfer_semaphore: &Arc<Semaphore>) {
        if matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
            return;
        }
        let tid = &self.task_id;
        let root_id = self.config.sync_root_id.clone();

        let mut upload_handles: Vec<(String, tokio::task::JoinHandle<Result<()>>)> = Vec::new();
        for action in &self.plan.uploads {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let action = action.clone();
            let tid_c = tid.clone();
            let config = self.config.clone();
            let api = self.api.clone();
            let db = self.db.clone();
            let file_locks = self.file_locks.clone();
            let ensured_dirs = self.ensured_dirs.clone();
            let sem = transfer_semaphore.clone();
            let root_id_c = root_id.clone();
            let rel_path = action.relative_path.clone();

            let handle = tokio::spawn(async move {
                crate::uploader::upload_file(
                    &tid_c,
                    &action,
                    &config,
                    &api,
                    &db,
                    &file_locks,
                    &ensured_dirs,
                    &sem,
                    &root_id_c,
                )
                .await
            });
            upload_handles.push((rel_path, handle));
        }

        for (rel_path, handle) in upload_handles {
            match handle.await {
                Ok(Ok(_)) => {
                    summary.uploaded += 1;
                    let _ = self.db.increment_task_completed(tid).await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "upload",
                            &TaskItemStatus::Completed,
                            None,
                        )
                        .await;
                }
                Ok(Err(e)) => {
                    tracing::error!("[{}] 上传失败: {}: {}", tid, rel_path, e);
                    summary.failed += 1;
                    let _ = self.db.increment_task_failed(tid).await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "upload",
                            &TaskItemStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
                Err(e) => {
                    tracing::error!("[{}] 上传任务异常: {}: {}", tid, rel_path, e);
                    summary.failed += 1;
                    let _ = self.db.increment_task_failed(tid).await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "upload",
                            &TaskItemStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        }
    }

    /// 5. 并发下载（DownloadOnly / Full）或 创建占位符（MirrorWcf）
    async fn step_execute_downloads_or_placeholders(&self, summary: &mut SyncSummary, transfer_semaphore: &Arc<Semaphore>) {
        let tid = &self.task_id;
        let root_id = self.config.sync_root_id.clone();

        if matches!(self.config.sync_mode, SyncMode::MirrorWcf) {
            // MirrorWcf: 为每个下载项创建占位符，而非实际下载
            for action in &self.plan.downloads {
                if self.shutdown_token.is_cancelled() {
                    break;
                }
                let relative = &action.relative_path;
                let local_path = self.config.local_root.join(relative);

                if let Some(parent) = local_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }

                if let Some(ref remote) = action.remote_entry {
                    if remote.is_dir {
                        let _ = tokio::fs::create_dir_all(&local_path).await;
                    } else {
                        #[cfg(feature = "windows-cfapi")]
                        {
                            let file_identity = serde_json::to_vec(&serde_json::json!({
                                "uri": remote.uri,
                                "size": remote.size,
                                "hash": remote.hash,
                                "mtime_ms": remote.mtime_ms,
                            }))
                            .unwrap_or_default();

                            if let Some(ref adapter) = self.platform_adapter {
                                match adapter
                                    .create_placeholder_file(
                                        local_path
                                            .parent()
                                            .unwrap_or(self.config.local_root.as_path()),
                                        local_path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default(),
                                        remote.size,
                                        &file_identity,
                                    )
                                    .await
                                {
                                    Ok(_) => tracing::debug!("[{}] 创建占位符: {}", tid, relative),
                                    Err(e) => {
                                        tracing::warn!(
                                            "[{}] 创建占位符失败，降级空文件 {}: {}",
                                            tid,
                                            relative,
                                            e
                                        );
                                        let _ = tokio::fs::write(&local_path, []).await;
                                    }
                                }
                            } else {
                                tracing::error!(
                                    "[{}] platform_adapter 未初始化，降级空文件: {}",
                                    tid,
                                    relative
                                );
                                let _ = tokio::fs::write(&local_path, []).await;
                            }
                        }
                        #[cfg(not(feature = "windows-cfapi"))]
                        {
                            let _ = tokio::fs::write(&local_path, []).await;
                            tracing::warn!(
                                "[{}] MirrorWcf 不可用，创建空文件降级: {}",
                                tid,
                                relative
                            );
                        }
                    }

                    let _ = self
                        .db
                        .upsert_file_mapping(&FileMapping {
                            id: 0,
                            sync_root_id: root_id.clone(),
                            local_path: std::path::PathBuf::from(relative),
                            remote_uri: remote.uri.clone(),
                            remote_file_id: remote.file_id.clone(),
                            local_hash: None,
                            remote_hash: remote.hash.clone(),
                            local_mtime: None,
                            remote_mtime: Some(remote.mtime_ms),
                            local_size: None,
                            remote_size: Some(remote.size),
                            sync_status: SyncFileStatus::Placeholder,
                            is_placeholder: true,
                        })
                        .await;
                }

                let _ = self.db.increment_task_completed(tid).await;
                let _ = self
                    .db
                    .update_task_item_status_by_path(
                        tid,
                        relative,
                        "create_placeholder",
                        &TaskItemStatus::Completed,
                        None,
                    )
                    .await;
            }
        } else if !matches!(self.config.sync_mode, SyncMode::UploadOnly) {
            let mut download_handles: Vec<(String, tokio::task::JoinHandle<Result<()>>)> =
                Vec::new();
            for action in &self.plan.downloads {
                if self.shutdown_token.is_cancelled() {
                    break;
                }
                let action = action.clone();
                let tid_c = tid.clone();
                let config = self.config.clone();
                let api = self.api.clone();
                let db = self.db.clone();
                let file_locks = self.file_locks.clone();
                let sem = transfer_semaphore.clone();
                let root_id_c = root_id.clone();
                let rel_path = action.relative_path.clone();

                let handle = tokio::spawn(async move {
                    crate::downloader::download_file(
                        &tid_c,
                        &action,
                        &config,
                        &api,
                        &db,
                        &file_locks,
                        &sem,
                        &root_id_c,
                    )
                    .await
                });
                download_handles.push((rel_path, handle));
            }

            for (rel_path, handle) in download_handles {
                match handle.await {
                    Ok(Ok(_)) => {
                        summary.downloaded += 1;
                        let _ = self.db.increment_task_completed(tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &rel_path,
                                "download",
                                &TaskItemStatus::Completed,
                                None,
                            )
                            .await;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("[{}] 下载失败: {}: {}", tid, rel_path, e);
                        summary.failed += 1;
                        let _ = self.db.increment_task_failed(tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &rel_path,
                                "download",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                    Err(e) => {
                        tracing::error!("[{}] 下载任务异常: {}: {}", tid, rel_path, e);
                        summary.failed += 1;
                        let _ = self.db.increment_task_failed(tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &rel_path,
                                "download",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        }
    }

    /// 6. 删除本地文件（DownloadOnly / Full / MirrorWcf — 远程删除触发的本地删除）
    async fn step_delete_local(&self, summary: &mut SyncSummary) {
        if matches!(self.config.sync_mode, SyncMode::UploadOnly) {
            return;
        }
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        for action in &self.plan.delete_local {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            if let Some(ref local) = action.local_entry {
                let local_path = self.config.local_root.join(&local.relative_path);
                match tokio::fs::remove_file(&local_path).await {
                    Ok(_) => {
                        summary.deleted_local += 1;
                        let _ = self.db.increment_task_completed(tid).await;
                        tracing::info!("[{}] 删除本地: {}", tid, action.relative_path);
                        let _ = self
                            .db
                            .delete_file_mapping(root_id, &action.relative_path)
                            .await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &action.relative_path,
                                "delete_local",
                                &TaskItemStatus::Completed,
                                None,
                            )
                            .await;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[{}] 删除本地文件失败 {}: {}",
                            tid,
                            action.relative_path,
                            e
                        );
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &action.relative_path,
                                "delete_local",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        }
    }

    /// 6.5 本地重命名（远程触发 → 本地执行 rename）
    async fn step_rename_local(&self, summary: &mut SyncSummary) {
        if matches!(self.config.sync_mode, SyncMode::UploadOnly) {
            return;
        }
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        for action in &self.plan.rename_local {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let old_path = self.config.local_root.join(&action.old_relative_path);
            let new_path = self.config.local_root.join(&action.new_relative_path);
            let rel_path = format!(
                "{} -> {}",
                action.old_relative_path, action.new_relative_path
            );

            if let Some(parent) = new_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            match tokio::fs::rename(&old_path, &new_path).await {
                Ok(_) => {
                    summary.renamed += 1;
                    let _ = self.db.increment_task_completed(tid).await;
                    tracing::info!(
                        "[{}] 本地重命名: {} -> {}",
                        tid,
                        action.old_relative_path,
                        action.new_relative_path
                    );
                    let _ = self
                        .db
                        .update_file_mapping_path(
                            root_id,
                            &action.old_relative_path,
                            &action.new_relative_path,
                            &action.new_remote_uri,
                        )
                        .await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "rename",
                            &TaskItemStatus::Completed,
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    summary.failed += 1;
                    tracing::warn!(
                        "[{}] 本地重命名失败: {} -> {}: {}",
                        tid,
                        action.old_relative_path,
                        action.new_relative_path,
                        e
                    );
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "rename",
                            &TaskItemStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        }
    }

    /// 6.6 本地移动（远程触发 → 本地执行 move）
    async fn step_move_local(&self, summary: &mut SyncSummary) {
        if matches!(self.config.sync_mode, SyncMode::UploadOnly) {
            return;
        }
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        for action in &self.plan.move_local {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let old_path = self.config.local_root.join(&action.old_relative_path);
            let new_path = self.config.local_root.join(&action.new_relative_path);
            let rel_path = format!(
                "{} -> {}",
                action.old_relative_path, action.new_relative_path
            );

            if let Some(parent) = new_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            match tokio::fs::rename(&old_path, &new_path).await {
                Ok(_) => {
                    summary.moved += 1;
                    let _ = self.db.increment_task_completed(tid).await;
                    tracing::info!(
                        "[{}] 本地移动: {} -> {}",
                        tid,
                        action.old_relative_path,
                        action.new_relative_path
                    );
                    let _ = self
                        .db
                        .update_file_mapping_path(
                            root_id,
                            &action.old_relative_path,
                            &action.new_relative_path,
                            &action.new_remote_uri,
                        )
                        .await;
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "move",
                            &TaskItemStatus::Completed,
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    summary.failed += 1;
                    tracing::warn!(
                        "[{}] 本地移动失败: {} -> {}: {}",
                        tid,
                        action.old_relative_path,
                        action.new_relative_path,
                        e
                    );
                    let _ = self
                        .db
                        .update_task_item_status_by_path(
                            tid,
                            &rel_path,
                            "move",
                            &TaskItemStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        }
    }

    /// 7. 删除远程文件（Full 和 MirrorWcf）
    async fn step_delete_remote(&self, summary: &mut SyncSummary) {
        if !matches!(self.config.sync_mode, SyncMode::Full | SyncMode::MirrorWcf) {
            return;
        }
        let tid = &self.task_id;
        let root_id = &self.config.sync_root_id;

        let remote_uris: Vec<&str> = self
            .plan
            .delete_remote
            .iter()
            .filter_map(|a| a.remote_entry.as_ref().map(|r| r.uri.as_str()))
            .collect();
        if !remote_uris.is_empty() {
            match self.api.delete_files(&remote_uris).await {
                Ok(_) => {
                    summary.deleted_remote += remote_uris.len() as u32;
                    for _ in &remote_uris {
                        let _ = self.db.increment_task_completed(tid).await;
                    }
                    for uri in &remote_uris {
                        tracing::info!("[{}] 删除远程: {}", tid, uri);
                    }
                    for action in &self.plan.delete_remote {
                        let _ = self
                            .db
                            .delete_file_mapping(root_id, &action.relative_path)
                            .await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &action.relative_path,
                                "delete_remote",
                                &TaskItemStatus::Completed,
                                None,
                            )
                            .await;
                    }
                }
                Err(e) => {
                    tracing::error!("[{}] 批量删除远程文件失败: {}", tid, e);
                    for action in &self.plan.delete_remote {
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                tid,
                                &action.relative_path,
                                "delete_remote",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        }
    }
}
