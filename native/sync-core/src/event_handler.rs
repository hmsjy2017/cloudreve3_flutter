use crate::api_client::ApiClient;
use crate::errors::Result;
use crate::models::RemoteFileEvent;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct EventHandler {
    api: Arc<ApiClient>,
    client_id: String,
}

impl EventHandler {
    pub fn new(api: Arc<ApiClient>, client_id: String) -> Self {
        Self { api, client_id }
    }

    /// 订阅服务端 SSE 事件流
    /// Phase 3 将实现完整的 SSE 解析
    pub async fn subscribe_sse(&self) -> Result<mpsc::Receiver<RemoteFileEvent>> {
        let (tx, rx) = mpsc::channel(256);
        // Phase 3: 实现 SSE 连接和事件解析
        Ok(rx)
    }
}

/// 事件防抖器：同一文件在 debounce_window 内的多次变更合并为一次
pub struct EventDebouncer {
    pending: HashMap<PathBuf, Instant>,
    debounce_window: Duration,
}

impl EventDebouncer {
    pub fn new(debounce_window: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            debounce_window,
        }
    }

    /// 推入事件路径，返回是否应该立即处理
    pub fn should_process(&mut self, path: &PathBuf) -> bool {
        let now = Instant::now();
        if let Some(last) = self.pending.get(path) {
            if now.duration_since(*last) < self.debounce_window {
                self.pending.insert(path.clone(), now);
                return false;
            }
        }
        self.pending.insert(path.clone(), now);
        true
    }

    /// 清理过期的防抖记录
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        self.pending.retain(|_, last| now.duration_since(*last) < self.debounce_window);
    }
}
