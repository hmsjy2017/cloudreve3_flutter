use crate::models::*;
use std::collections::{HashMap, HashSet};

/// 三路差异计算: 本地 vs 远程 vs 数据库，按同步模式过滤
pub fn compute_diff(
    local_files: &[LocalFileEntry],
    remote_files: &[RemoteFileEntry],
    db_mappings: &HashMap<String, FileMapping>,
    remote_root: &str,
    sync_mode: &SyncMode,
) -> SyncPlan {
    let mut plan = SyncPlan::default();

    // 构建索引: relative_path → entry（统一正斜杠）
    let local_map: HashMap<String, &LocalFileEntry> = local_files
        .iter()
        .map(|e| (crate::utils::normalize_path(&e.relative_path.to_string_lossy()), e))
        .collect();

    let remote_map: HashMap<String, &RemoteFileEntry> = remote_files
        .iter()
        .map(|e| {
            let rel = remote_relative_path(remote_root, &e.path, &e.name, e.is_dir);
            (rel, e)
        })
        .collect();

    // 收集所有路径
    let mut all_paths: HashSet<String> = HashSet::new();
    for k in local_map.keys() {
        all_paths.insert(k.clone());
    }
    for k in remote_map.keys() {
        all_paths.insert(k.clone());
    }
    for k in db_mappings.keys() {
        all_paths.insert(k.clone());
    }

    for path in &all_paths {
        let local = local_map.get(path.as_str()).copied();
        let remote = remote_map.get(path.as_str()).copied();
        let db = db_mappings.get(path.as_str());

        match (local, remote, db) {
            // 本地有，远程无 → 上传（UploadOnly 和 Full）
            (Some(l), None, _) => {
                if matches!(sync_mode, SyncMode::DownloadOnly) {
                    continue;
                }
                if !l.is_dir && l.size == 0 {
                    continue;
                }
                if let Some(db_m) = db {
                    if db_m.sync_status == SyncFileStatus::Synced {
                        plan.uploads.push(SyncAction {
                            relative_path: path.clone(),
                            local_entry: Some((*l).clone()),
                            remote_entry: None,
                            db_mapping: Some(db_m.clone()),
                        });
                    }
                } else {
                    plan.uploads.push(SyncAction {
                        relative_path: path.clone(),
                        local_entry: Some((*l).clone()),
                        remote_entry: None,
                        db_mapping: None,
                    });
                }
            }

            // 远程有，本地无 → 下载（DownloadOnly 和 Full）
            (None, Some(r), _) => {
                if matches!(sync_mode, SyncMode::UploadOnly) {
                    continue;
                }
                if r.is_dir {
                    plan.mkdirs_local.push(path.clone());
                } else if r.size == 0 {
                    continue;
                } else {
                    plan.downloads.push(SyncAction {
                        relative_path: path.clone(),
                        local_entry: None,
                        remote_entry: Some((*r).clone()),
                        db_mapping: db.cloned(),
                    });
                }
            }

            // 两边都有
            (Some(l), Some(r), db_m) => {
                if l.is_dir && r.is_dir {
                    continue;
                }

                let content_match = match (&l.quick_hash, &r.hash) {
                    (lh, Some(rh)) if !lh.is_empty() && !rh.is_empty() => lh == rh,
                    _ => l.size == r.size,
                };

                if content_match {
                    // 内容一致，标记已同步
                } else {
                    let conflict_type = if l.is_dir != r.is_dir {
                        ConflictType::TypeMismatch
                    } else {
                        ConflictType::BothModified
                    };

                    plan.conflicts.push(SyncConflict {
                        relative_path: path.clone(),
                        conflict_type,
                        local_entry: Some((*l).clone()),
                        remote_entry: Some((*r).clone()),
                        db_mapping: db_m.cloned(),
                    });
                }
            }

            (None, None, Some(_)) => {}

            _ => {}
        }
    }

    // 远程目录结构（仅 UploadOnly 和 Full）
    if !matches!(sync_mode, SyncMode::DownloadOnly) {
        for (path, local) in &local_map {
            if local.is_dir && !remote_map.contains_key(path.as_str()) {
                plan.mkdirs_remote.push(path.clone());
            }
        }
    }

    // 按 sync_mode 自动解决冲突
    resolve_conflicts_by_mode(&mut plan, sync_mode);

    plan
}

/// 根据同步模式自动解决冲突
fn resolve_conflicts_by_mode(plan: &mut SyncPlan, sync_mode: &SyncMode) {
    if plan.conflicts.is_empty() {
        return;
    }

    let conflicts = std::mem::take(&mut plan.conflicts);
    for conflict in conflicts {
        match sync_mode {
            SyncMode::UploadOnly => {
                // 冲突一律覆盖上传
                let action = SyncAction {
                    relative_path: conflict.relative_path.clone(),
                    local_entry: conflict.local_entry.clone(),
                    remote_entry: conflict.remote_entry.clone(),
                    db_mapping: conflict.db_mapping.clone(),
                };
                plan.uploads.push(action);
            }
            SyncMode::DownloadOnly => {
                // 冲突一律覆盖下载
                let action = SyncAction {
                    relative_path: conflict.relative_path.clone(),
                    local_entry: conflict.local_entry.clone(),
                    remote_entry: conflict.remote_entry.clone(),
                    db_mapping: conflict.db_mapping.clone(),
                };
                plan.downloads.push(action);
            }
            _ => {
                // Full: 保留冲突，由 Worker 用 conflict_strategy 解决
                plan.conflicts.push(conflict);
            }
        }
    }
}

/// 从远程 path 字段提取相对路径
pub fn remote_relative_path(remote_root: &str, path: &str, name: &str, is_dir: bool) -> String {
    let _ = is_dir;
    // 1. 尝试匹配完整 URI 前缀（remote_root = cloudreve://my/example）
    if let Some(rel) = path.strip_prefix(remote_root) {
        return rel.trim_start_matches('/').to_string();
    }
    // 2. SSE 的 path 是相对路径（如 /Readest/Books/file.txt），直接 trim 前导 /
    if path.starts_with('/') {
        return path.trim_start_matches('/').to_string();
    }
    // 3. 回退：使用文件名
    name.to_string()
}

/// 从字符串解析 SyncFileStatus
pub fn parse_sync_status_from_str(s: &str) -> SyncFileStatus {
    match s {
        "uploading" => SyncFileStatus::Uploading,
        "downloading" => SyncFileStatus::Downloading,
        "conflict" => SyncFileStatus::Conflict,
        "placeholder" => SyncFileStatus::Placeholder,
        _ => SyncFileStatus::Synced,
    }
}
