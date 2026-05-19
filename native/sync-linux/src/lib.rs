//! Linux 平台适配层
//!
//! 提供文件系统监听 (notify/inotify) 用于：
//! - 实时检测本地文件变更
//! - inotify 限制检测与配置
//! - 混合模式（事件+轮询）切换

use std::path::Path;

pub struct LinuxAdapter {
    watcher: Option<notify::RecommendedWatcher>,
}

impl Default for LinuxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxAdapter {
    pub fn new() -> Self {
        Self { watcher: None }
    }

    /// 启动文件监听 (Phase 3)
    pub fn start_watching(&mut self, _root: &Path) -> anyhow::Result<()> {
        // Phase 3: 实现 notify 监听
        Ok(())
    }

    /// 停止监听
    pub fn stop_watching(&mut self) -> anyhow::Result<()> {
        if let Some(w) = self.watcher.take() {
            drop(w);
        }
        Ok(())
    }
}

/// inotify 限制检测
pub struct InotifyConfig;

impl InotifyConfig {
    pub fn check_limits() -> InotifyStatus {
        let max_watches = std::fs::read_to_string("/proc/sys/fs/inotify/max_user_watches")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(8192);

        let max_user_instances = std::fs::read_to_string("/proc/sys/fs/inotify/max_user_instances")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(128);

        InotifyStatus {
            max_watches,
            max_user_instances,
            recommendation: if max_watches < 524288 {
                Some("建议执行: echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf && sudo sysctl -p".to_string())
            } else {
                None
            },
        }
    }
}

pub struct InotifyStatus {
    pub max_watches: u64,
    pub max_user_instances: u64,
    pub recommendation: Option<String>,
}

/// 占位符信息 (Linux 不使用，保持接口一致)
pub struct PlaceholderEntry {
    pub relative_path: String,
    pub file_size: u64,
    pub is_dir: bool,
}
