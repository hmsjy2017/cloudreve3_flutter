use crate::errors::Result;
use crate::models::LocalFileEntry;
use crate::utils::quick_hash;
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

pub struct FsScanner;

impl FsScanner {
    pub fn new() -> Self {
        Self
    }

    /// 递归扫描本地目录
    pub async fn scan(
        &self,
        root: &Path,
        depth_limit: u32,
        follow_symlinks: bool,
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

            // 符号链接处理
            if entry.path_is_symlink() && !follow_symlinks {
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

            if metadata.is_dir() {
                entries.push(LocalFileEntry {
                    relative_path,
                    size: 0,
                    mtime_ms: 0,
                    quick_hash: String::new(),
                    is_dir: true,
                });
            } else if metadata.is_file() {
                let size = metadata.len();
                let mtime_ms = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);

                let hash = quick_hash(entry.path(), size).await.unwrap_or_default();

                entries.push(LocalFileEntry {
                    relative_path,
                    size,
                    mtime_ms,
                    quick_hash: hash,
                    is_dir: false,
                });
            }
        }

        Ok(entries)
    }
}
