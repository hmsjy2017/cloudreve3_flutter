use crate::api_client::ApiClient;
use crate::conflict_resolver::ConflictResolver;
use crate::errors::{Result, SyncError};
use crate::file_lock::FileLockRegistry;
use crate::models::*;
use crate::sync_db::SyncDb;
use dashmap::DashMap;
use dashmap::DashSet;
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

        // 更新 DB: status = running
        let _ = self
            .db
            .update_sync_task_status(&tid, &WorkerStatus::Running)
            .await;

        // 创建本地传输并发信号量
        let transfer_semaphore =
            Arc::new(Semaphore::new(self.config.max_concurrent_transfers.max(1)));

        let mut summary = SyncSummary::default();
        let root_id = self.config.sync_root_id.clone();

        // 1. 创建远程目录结构（UploadOnly / Full）
        if !matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
            for dir_path in &self.plan.mkdirs_remote {
                if self.shutdown_token.is_cancelled() {
                    break;
                }
                match crate::uploader::ensure_remote_dirs(
                    &tid,
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

        // 2. 创建本地目录结构（DownloadOnly / Full）
        if !matches!(self.config.sync_mode, SyncMode::UploadOnly) {
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

        // 2.1 执行远程重命名（仅 UploadOnly / Full）
        if !matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
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
                        let _ = self.db.increment_task_completed(&tid).await;
                        let new_remote_uri = {
                            let uri = &rename.remote_uri;
                            let last_slash = uri.trim_end_matches('/').rfind('/').unwrap_or(0);
                            format!("{}/{}", &uri[..last_slash], rename.new_name)
                        };
                        let _ = self
                            .db
                            .update_file_mapping_path(
                                &root_id,
                                &rename.old_relative_path,
                                &rename.new_relative_path,
                                &new_remote_uri,
                            )
                            .await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                                &tid,
                                &rel_path,
                                "rename",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }

            // 2.2 执行远程移动（本地触发）
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
                        let _ = self.db.increment_task_completed(&tid).await;
                        let _ = self
                            .db
                            .update_file_mapping_path(
                                &root_id,
                                &mov.old_relative_path,
                                &mov.new_relative_path,
                                &mov.dst_remote_dir_uri,
                            )
                            .await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                                &tid,
                                &rel_path,
                                "move",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        } // end UploadOnly/Full check for rename+move

        // 2.5 递归扫描 scan_dirs 中的目录，将文件加入 uploads（UploadOnly / Full）
        if !self.plan.scan_dirs.is_empty() {
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
                            // 跳过 size=0 的普通文件
                            if !entry.is_dir && entry.size == 0 {
                                tracing::debug!(
                                    "[{}] 跳过空文件: {}",
                                    tid,
                                    entry.relative_path.display()
                                );
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
                            &tid,
                            &action,
                            &self.config,
                            &self.api,
                            &self.db,
                            &self.file_locks,
                            &self.ensured_dirs,
                            &transfer_semaphore,
                            &root_id,
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
                            &tid,
                            &action,
                            &self.config,
                            &self.api,
                            &self.db,
                            &self.file_locks,
                            &transfer_semaphore,
                            &root_id,
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
                                    &tid,
                                    &action,
                                    &self.config,
                                    &self.api,
                                    &self.db,
                                    &self.file_locks,
                                    &transfer_semaphore,
                                    &root_id,
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
            // 更新冲突 task_item 状态
            let item_status = if conflict_ok {
                TaskItemStatus::Completed
            } else {
                TaskItemStatus::Failed
            };
            if conflict_ok {
                let _ = self.db.increment_task_completed(&tid).await;
            }
            let _ = self
                .db
                .update_task_item_status_by_path(
                    &tid,
                    &conflict.relative_path,
                    "conflict_resolve",
                    &item_status,
                    None,
                )
                .await;
        }

        // 4. 并发上传（UploadOnly / Full）
        if !matches!(self.config.sync_mode, SyncMode::DownloadOnly) {
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
                        let _ = self.db.increment_task_completed(&tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                        let _ = self.db.increment_task_failed(&tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                        let _ = self.db.increment_task_failed(&tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
                                &rel_path,
                                "upload",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        } // end UploadOnly/Full check for uploads

        // 5. 并发下载（DownloadOnly / Full）
        if !matches!(self.config.sync_mode, SyncMode::UploadOnly) {
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
                        let _ = self.db.increment_task_completed(&tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                        let _ = self.db.increment_task_failed(&tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                        let _ = self.db.increment_task_failed(&tid).await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
                                &rel_path,
                                "download",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        } // end DownloadOnly/Full check for downloads

        // 6. 删除本地文件（DownloadOnly / Full — 远程删除触发的本地删除）
        if !matches!(self.config.sync_mode, SyncMode::UploadOnly) {
            for action in &self.plan.delete_local {
                if self.shutdown_token.is_cancelled() {
                    break;
                }
                if let Some(ref local) = action.local_entry {
                    let local_path = self.config.local_root.join(&local.relative_path);
                    match tokio::fs::remove_file(&local_path).await {
                        Ok(_) => {
                            summary.deleted_local += 1;
                            let _ = self.db.increment_task_completed(&tid).await;
                            tracing::info!("[{}] 删除本地: {}", tid, action.relative_path);
                            let _ = self
                                .db
                                .delete_file_mapping(&root_id, &action.relative_path)
                                .await;
                            let _ = self
                                .db
                                .update_task_item_status_by_path(
                                    &tid,
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
                                    &tid,
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
        } // end DownloadOnly/Full check for delete_local

        // 6.5 本地重命名（远程触发 → 本地执行 rename）
        if !matches!(self.config.sync_mode, SyncMode::UploadOnly) {
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
                        let _ = self.db.increment_task_completed(&tid).await;
                        tracing::info!(
                            "[{}] 本地重命名: {} -> {}",
                            tid,
                            action.old_relative_path,
                            action.new_relative_path
                        );
                        let _ = self
                            .db
                            .update_file_mapping_path(
                                &root_id,
                                &action.old_relative_path,
                                &action.new_relative_path,
                                &action.new_remote_uri,
                            )
                            .await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                                &tid,
                                &rel_path,
                                "rename",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        } // end UploadOnly check for rename_local

        // 6.6 本地移动（远程触发 → 本地执行 move）
        if !matches!(self.config.sync_mode, SyncMode::UploadOnly) {
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
                        let _ = self.db.increment_task_completed(&tid).await;
                        tracing::info!(
                            "[{}] 本地移动: {} -> {}",
                            tid,
                            action.old_relative_path,
                            action.new_relative_path
                        );
                        let _ = self
                            .db
                            .update_file_mapping_path(
                                &root_id,
                                &action.old_relative_path,
                                &action.new_relative_path,
                                &action.new_remote_uri,
                            )
                            .await;
                        let _ = self
                            .db
                            .update_task_item_status_by_path(
                                &tid,
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
                                &tid,
                                &rel_path,
                                "move",
                                &TaskItemStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
        } // end UploadOnly check for move_local

        // 7. 删除远程文件（仅 Full）
        if matches!(self.config.sync_mode, SyncMode::Full) {
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
                            let _ = self.db.increment_task_completed(&tid).await;
                        }
                        for uri in &remote_uris {
                            tracing::info!("[{}] 删除远程: {}", tid, uri);
                        }
                        for action in &self.plan.delete_remote {
                            let _ = self
                                .db
                                .delete_file_mapping(&root_id, &action.relative_path)
                                .await;
                            let _ = self
                                .db
                                .update_task_item_status_by_path(
                                    &tid,
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
                                    &tid,
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
        } // end Full check for delete_remote

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

        // 推送事件到 Dart
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
}

/// WorkerPool — 全局 Worker 并发控制
pub struct WorkerPool {
    worker_semaphore: Arc<Semaphore>,
    active_workers: Arc<DashMap<String, tokio::task::JoinHandle<()>>>,
    /// 当前正在上传的相对路径集合，用于去重
    active_upload_paths: Arc<DashSet<String>>,
    /// 活跃 worker 总数（包括 submit 阻塞型 + submit_background 后台型）
    active_count: Arc<std::sync::atomic::AtomicU32>,
    db: Arc<SyncDb>,
    api: Arc<ApiClient>,
    file_locks: Arc<FileLockRegistry>,
    ensured_dirs: Arc<DashMap<String, ()>>,
    event_sink: Arc<crate::event_sink::EventSink>,
    shutdown_token: std::sync::Mutex<CancellationToken>,
}

impl WorkerPool {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Arc<SyncDb>,
        api: Arc<ApiClient>,
        file_locks: Arc<FileLockRegistry>,
        ensured_dirs: Arc<DashMap<String, ()>>,
        event_sink: Arc<crate::event_sink::EventSink>,
        shutdown_token: CancellationToken,
        max_workers_override: usize,
        client_id: &str,
    ) -> Self {
        let cpu_count = num_cpus();
        let max_workers = if max_workers_override > 0 {
            max_workers_override.min(cpu_count * 2).max(1)
        } else {
            cpu_count.clamp(1, 32)
        };
        tracing::info!(
            "WorkerPool 初始化: 最大并发 Worker 数={} (cpu={}, override={})",
            max_workers,
            cpu_count,
            max_workers_override
        );
        tracing::info!("Client ID: {}", client_id);

        Self {
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: Arc::new(DashMap::new()),
            active_upload_paths: Arc::new(DashSet::new()),
            active_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            db,
            api,
            file_locks,
            ensured_dirs,
            event_sink,
            shutdown_token: std::sync::Mutex::new(shutdown_token),
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
        // 空 plan 直接返回，不创建任务记录
        if !plan.has_work() {
            tracing::debug!("SyncPlan 无操作，跳过 Worker 创建");
            return Ok(SyncSummary::default());
        }

        let task_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // 创建 DB 任务记录
        let total_count = plan.uploads.len() as u32
            + plan.downloads.len() as u32
            + plan.delete_local.len() as u32
            + plan.delete_remote.len() as u32
            + plan.rename_remote.len() as u32
            + plan.move_remote.len() as u32
            + plan.rename_local.len() as u32
            + plan.move_local.len() as u32
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
        self.create_task_items(&task_id, &plan, &now_for_items)
            .await?;

        // 等待 Worker 信号量
        let _permit = self
            .worker_semaphore
            .acquire()
            .await
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
            self.shutdown_token.lock().unwrap().clone(),
        );

        // 推送 WorkerStarted 事件
        let _ = self
            .event_sink
            .emit(crate::api::ffi_types::SyncEventFfi::WorkerStarted {
                task_id: task_id.clone(),
                trigger: task.trigger.as_str().to_string(),
                upload_count: task.total_count,
                download_count: 0,
            })
            .await;

        self.active_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let result = worker.run().await;
        self.active_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        result
    }

    /// 提交 Worker（火力全忘，后台运行）
    pub async fn submit_background(
        &self,
        plan: SyncPlan,
        config: WorkerConfig,
        trigger: WorkerTrigger,
        conflict_resolver: ConflictResolver,
    ) -> Option<String> {
        // 空 plan 直接返回，不创建任务记录
        if !plan.has_work() {
            return None;
        }

        let task_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let total_count = plan.uploads.len() as u32
            + plan.downloads.len() as u32
            + plan.delete_local.len() as u32
            + plan.delete_remote.len() as u32
            + plan.rename_remote.len() as u32
            + plan.move_remote.len() as u32
            + plan.rename_local.len() as u32
            + plan.move_local.len() as u32
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
        if let Err(e) = self
            .create_task_items(&task_id, &plan, &now_for_items)
            .await
        {
            tracing::warn!("创建任务项记录失败: {}", e);
        }

        let tid = task_id.clone();
        let trigger_str = trigger.as_str().to_string();
        let upload_count = plan.uploads.len() as u32;
        let download_count = plan.downloads.len() as u32;

        // 推送 WorkerStarted 事件
        let _ = self
            .event_sink
            .emit(crate::api::ffi_types::SyncEventFfi::WorkerStarted {
                task_id: tid.clone(),
                trigger: trigger_str,
                upload_count,
                download_count,
            })
            .await;

        // 注册上传路径到去重集合
        self.register_upload_paths(&plan);
        let upload_paths_for_cleanup: Vec<String> = plan
            .uploads
            .iter()
            .map(|a| a.relative_path.clone())
            .collect();

        let sem = self.worker_semaphore.clone();
        let db = self.db.clone();
        let api = self.api.clone();
        let file_locks = self.file_locks.clone();
        let ensured_dirs = self.ensured_dirs.clone();
        let event_sink = self.event_sink.clone();
        let shutdown_token = self.shutdown_token.lock().unwrap().clone();
        let active_workers = self.active_workers.clone();
        let active_upload_paths = self.active_upload_paths.clone();
        let active_count = self.active_count.clone();

        self.active_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

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
            // Worker 完成后从活跃列表和上传去重集合移除
            active_workers.remove(&task_id);
            for path in &upload_paths_for_cleanup {
                active_upload_paths.remove(path);
            }
            active_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        });

        self.active_workers.insert(tid.clone(), handle);
        Some(tid)
    }

    /// 当前活跃 Worker 数（含阻塞型和后台型）
    pub fn active_worker_count(&self) -> usize {
        self.active_count.load(std::sync::atomic::Ordering::Relaxed) as usize
    }

    /// 检查指定相对路径是否正在上传中
    pub fn is_uploading(&self, relative_path: &str) -> bool {
        self.active_upload_paths.contains(relative_path)
    }

    /// 注册上传路径到去重集合
    fn register_upload_paths(&self, plan: &SyncPlan) {
        for action in &plan.uploads {
            self.active_upload_paths
                .insert(action.relative_path.clone());
        }
    }

    /// 更新 shutdown token（引擎重启时调用）
    pub fn update_shutdown_token(&self, token: CancellationToken) {
        *self.shutdown_token.lock().unwrap() = token;
    }

    /// 终止所有活跃 Worker 并等待退出
    pub async fn abort_all_workers(&self) {
        // 收集所有 JoinHandle 的 ID
        let ids: Vec<String> = self
            .active_workers
            .iter()
            .map(|e| e.key().clone())
            .collect();
        // 逐个 abort
        for id in ids {
            if let Some((_, handle)) = self.active_workers.remove(&id) {
                handle.abort();
                let _ = handle.await;
            }
        }
        // 清空上传去重集合和计数器
        self.active_upload_paths.clear();
        self.active_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// 创建 task_item 记录（批量插入，单次持锁）
    async fn create_task_items(&self, task_id: &str, plan: &SyncPlan, now: &str) -> Result<()> {
        let mut items: Vec<SyncTaskItem> = Vec::new();

        for action in &plan.uploads {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::Upload,
                status: TaskItemStatus::Pending,
                file_size: action.local_entry.as_ref().map(|l| l.size).unwrap_or(0),
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for action in &plan.downloads {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::Download,
                status: TaskItemStatus::Pending,
                file_size: action.remote_entry.as_ref().map(|r| r.size).unwrap_or(0),
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for action in &plan.delete_local {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::DeleteLocal,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for action in &plan.delete_remote {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: action.relative_path.clone(),
                action_type: TaskActionType::DeleteRemote,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for rename in &plan.rename_remote {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: format!(
                    "{} -> {}",
                    rename.old_relative_path, rename.new_relative_path
                ),
                action_type: TaskActionType::Rename,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for mov in &plan.move_remote {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: format!("{} -> {}", mov.old_relative_path, mov.new_relative_path),
                action_type: TaskActionType::Move,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for conflict in &plan.conflicts {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: conflict.relative_path.clone(),
                action_type: TaskActionType::ConflictResolve,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for action in &plan.rename_local {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: format!(
                    "{} -> {}",
                    action.old_relative_path, action.new_relative_path
                ),
                action_type: TaskActionType::Rename,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }
        for action in &plan.move_local {
            items.push(SyncTaskItem {
                id: 0,
                task_id: task_id.to_string(),
                relative_path: format!(
                    "{} -> {}",
                    action.old_relative_path, action.new_relative_path
                ),
                action_type: TaskActionType::Move,
                status: TaskItemStatus::Pending,
                file_size: 0,
                error_message: None,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            });
        }

        self.db.create_sync_task_items_batch(&items).await
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
