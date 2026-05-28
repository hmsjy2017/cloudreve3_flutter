#[cfg(unix)]
use std::collections::HashSet;
use crate::errors::Result;
use crate::models::LocalFileEntry;
use crate::utils::quick_hash;
use std::path::Path;
use walkdir::WalkDir;

/// 需要跳过的文件/目录名前缀和名称
pub const SKIP_NAMES: &[&str] = &[
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
];

/// 需要跳过的文件扩展名（同步临时文件）
pub const SKIP_EXTENSIONS: &[&str] = &[
    "sync_tmp",
    "sync_temp",
];

pub struct FsScanner;

impl Default for FsScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl FsScanner {
    pub fn new() -> Self {
        Self
    }

    /// 递归扫描本地目录
    /// `compute_hash`: 是否计算文件 quick_hash（MirrorWcf 模式下可跳过以加速扫描）
    pub async fn scan(
        &self,
        root: &Path,
        depth_limit: u32,
        follow_symlinks: bool,
        compute_hash: bool,
    ) -> Result<Vec<LocalFileEntry>> {
        let mut entries = Vec::new();
        #[cfg(unix)]
        let mut visited_inodes: HashSet<(u64, u64)> = HashSet::new();

        let walker = WalkDir::new(root)
            .max_depth(depth_limit as usize)
            .follow_links(follow_symlinks);

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("扫描跳过: {}", e);
                    continue;
                }
            };

            let file_name = entry.file_name().to_string_lossy();
            let depth = entry.depth();
            tracing::trace!("扫描: depth={}, is_dir={}, name={}", depth, entry.file_type().is_dir(), file_name);

            // 符号链接处理
            if entry.path_is_symlink() && !follow_symlinks {
                continue;
            }

            // 跳过同步元数据文件和系统文件
            let file_name = entry.file_name().to_string_lossy();
            if SKIP_NAMES.iter().any(|s| file_name == *s) {
                continue;
            }
            // 跳过隐藏目录/文件（以 . 开头）
            if file_name.starts_with('.') {
                continue;
            }
            if file_name.starts_with(".sync_") {
                continue;
            }
            // 跳过临时文件扩展名
            if let Some(ext) = entry.path().extension() {
                if SKIP_EXTENSIONS.iter().any(|e| ext == *e) {
                    continue;
                }
            }
            // 跳过冲突副本文件
            if crate::utils::is_conflict_file(&file_name) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("无法读取元数据 {}: {}", entry.path().display(), e);
                    continue;
                }
            };

            // inode 去重（防止硬链接/符号链接循环）
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let key = (metadata.dev(), metadata.ino());
                if !visited_inodes.insert(key) {
                    continue;
                }
            }

            let relative_path = entry.path().strip_prefix(root)
                .unwrap_or(entry.path())
                .to_path_buf();

            // 跳过根目录自身（relative_path 为空）
            if relative_path.to_string_lossy().is_empty() {
                continue;
            }

            if metadata.is_dir() {
                entries.push(LocalFileEntry {
                    relative_path,
                    size: 0,
                    mtime_ms: 0,
                    quick_hash: String::new(),
                    is_dir: true,
                    mime_type: None,
                });
            } else if metadata.is_file() {
                let size = metadata.len();
                let mtime_ms = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);

                let hash = if compute_hash {
                    quick_hash(entry.path(), size).await.unwrap_or_default()
                } else {
                    String::new()
                };
                let mime_type = guess_mime_type(entry.path());

                entries.push(LocalFileEntry {
                    relative_path,
                    size,
                    mtime_ms,
                    quick_hash: hash,
                    is_dir: false,
                    mime_type,
                });
            }
        }

        let dirs = entries.iter().filter(|e| e.is_dir).count();
        let files = entries.iter().filter(|e| !e.is_dir).count();
        tracing::debug!("扫描完成: {} 个条目 ({} 目录, {} 文件)", entries.len(), dirs, files);

        Ok(entries)
    }
}

/// 根据文件扩展名推断 MIME 类型
pub fn guess_mime_type(path: &Path) -> Option<String> {
    mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
}
