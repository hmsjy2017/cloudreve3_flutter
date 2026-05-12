use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::errors::{Result, SyncError};

/// 增量哈希：前 8KB + 文件大小（快速判断文件是否变更）
pub async fn quick_hash(path: &Path, size: u64) -> Result<String> {
    if size == 0 {
        return Ok("0:0:0".to_string());
    }
    let file = tokio::fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut head = vec![0u8; 8192.min(size as usize)];
    let n = tokio::io::AsyncReadExt::read(&mut reader, &mut head).await?;
    head.truncate(n);
    let mut hasher = Sha256::new();
    hasher.update(&head);
    let hash = hasher.finalize();
    Ok(format!("{}:{}:{}", hex::encode(hash), n, size))
}

/// 验证路径在同步根目录下，防止路径遍历攻击
pub fn validate_path(root: &Path, path: &Path) -> Result<()> {
    let canonical_root = root.canonicalize().map_err(|e| SyncError::FileSystem(
        format!("无法解析根目录: {}", e)
    ))?;

    let canonical_path = path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());

    let relative = canonical_path.strip_prefix(&canonical_root)
        .map_err(|_| SyncError::PathTraversal {
            path: path.to_string_lossy().to_string(),
            root: canonical_root.to_string_lossy().to_string(),
        })?;

    if relative.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(SyncError::PathTraversal {
            path: path.to_string_lossy().to_string(),
            root: canonical_root.to_string_lossy().to_string(),
        });
    }

    Ok(())
}

/// 路径规范化：统一使用正斜杠，去除尾部斜杠
pub fn normalize_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_end_matches('/').to_string()
}

/// 生成冲突副本名称
pub fn generate_conflict_name(original: &str) -> String {
    let path = PathBuf::from(original);
    let stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let date = chrono::Local::now().format("%Y-%m-%d");
    if ext.is_empty() {
        format!("{} (冲突副本 {})", stem, date)
    } else {
        format!("{} (冲突副本 {}).{}", stem, date, ext)
    }
}

/// 计算重试延迟（指数退避）
pub fn retry_delay_ms(attempt: u32, base_ms: u64, max_ms: u64) -> u64 {
    let delay = base_ms * 2u64.saturating_pow(attempt);
    delay.min(max_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("foo\\bar\\baz"), "foo/bar/baz");
        assert_eq!(normalize_path("foo/bar/"), "foo/bar");
        assert_eq!(normalize_path("/foo/bar"), "/foo/bar");
    }

    #[test]
    fn test_generate_conflict_name() {
        let name = generate_conflict_name("photo.jpg");
        assert!(name.starts_with("photo (冲突副本 "));
        assert!(name.ends_with(".jpg"));

        let name_no_ext = generate_conflict_name("README");
        assert!(name_no_ext.starts_with("README (冲突副本 "));
    }

    #[test]
    fn test_retry_delay() {
        assert_eq!(retry_delay_ms(0, 1000, 60000), 1000);
        assert_eq!(retry_delay_ms(1, 1000, 60000), 2000);
        assert_eq!(retry_delay_ms(5, 1000, 60000), 32000);
        assert_eq!(retry_delay_ms(10, 1000, 60000), 60000);
    }
}
