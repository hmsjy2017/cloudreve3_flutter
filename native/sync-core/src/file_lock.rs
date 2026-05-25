use dashmap::DashMap;
use std::sync::Arc;
use std::sync::Weak;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// 文件锁注册表 — 防止同一文件被并发操作（如上传中删除）
pub struct FileLockRegistry {
    locks: DashMap<String, Arc<Mutex<()>>>,
}

impl Default for FileLockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLockRegistry {
    pub fn new() -> Self {
        Self {
            locks: DashMap::new(),
        }
    }

    /// 阻塞等待获取文件锁，返回守卫
    pub async fn acquire(&self, path: &str) -> FileLockGuard<'_> {
        let lock = self
            .locks
            .entry(path.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let guard = lock.lock_owned().await;
        FileLockGuard {
            registry: self,
            path: path.to_string(),
            _guard: guard,
        }
    }

    /// 移除无人等待的锁条目（守卫 Drop 时调用）
    fn cleanup_if_unused(&self, path: &str) {
        if let Some(entry) = self.locks.get(path) {
            if Arc::strong_count(entry.value()) == 1 {
                drop(entry);
                self.locks.remove(path);
            }
        }
    }

    /// 供外部引用计数判断
    pub fn get_weak(&self, path: &str) -> Weak<Mutex<()>> {
        let lock = self
            .locks
            .entry(path.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        Arc::downgrade(&lock)
    }
}

/// 文件锁守卫 — Drop 时自动清理无人等待的锁条目
pub struct FileLockGuard<'a> {
    registry: &'a FileLockRegistry,
    path: String,
    _guard: OwnedMutexGuard<()>,
}

impl Drop for FileLockGuard<'_> {
    fn drop(&mut self) {
        self.registry.cleanup_if_unused(&self.path);
    }
}
