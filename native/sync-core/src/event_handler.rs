use crate::api_client::ApiClient;
use crate::errors::Result;
use crate::models::{RemoteFileEntry, RemoteFileEvent};
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

    /// 订阅服务端 SSE 事件流 (GET /file/events)
    pub async fn subscribe_sse(&self, uri: &str) -> Result<mpsc::Receiver<RemoteFileEvent>> {
        let (tx, rx) = mpsc::channel(256);

        let base_url = self.api.base_url();
        let url = format!("{}/file/events", base_url);
        let token = self.api.token().await;
        let client_id = self.client_id.clone();
        let uri = uri.to_string();

        tokio::spawn(async move {
            loop {
                match Self::connect_sse(&url, &token, &client_id, &uri, &tx).await {
                    Ok(_) => {
                        tracing::info!("SSE 连接关闭，3秒后重连...");
                    }
                    Err(e) => {
                        tracing::warn!("SSE 连接错误: {}，5秒后重连...", e);
                    }
                }

                // 重连前等待
                tokio::time::sleep(Duration::from_secs(5)).await;

                // 如果接收端已关闭，退出循环
                if tx.is_closed() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    /// 建立 SSE 连接并解析事件
    async fn connect_sse(
        url: &str,
        token: &str,
        client_id: &str,
        _uri: &str,
        tx: &mpsc::Sender<RemoteFileEvent>,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .bearer_auth(token)
            .header("X-Cr-Client-Id", client_id)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .query(&[("uri", "cloudreve://my")])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(crate::errors::SyncError::Network(
                format!("SSE 连接失败: HTTP {}", resp.status()),
            ));
        }

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;

        let mut event_type = String::new();
        let mut data_buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| crate::errors::SyncError::Network(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(stripped) = line.strip_prefix("event:") {
                    event_type = stripped.trim().to_string();
                } else if let Some(stripped) = line.strip_prefix("data:") {
                    data_buffer = stripped.trim().to_string();
                } else if line.is_empty() && !data_buffer.is_empty() {
                    // 空行表示事件结束
                    if event_type == "event" {
                        if let Ok(events) = serde_json::from_str::<Vec<SseFileEvent>>(&data_buffer) {
                            for ev in events {
                                let remote_event = match ev.event_type.as_str() {
                                    "create" | "modify" => {
                                        // 需要获取文件详情
                                        Some(RemoteFileEvent::Modified(RemoteFileEntry {
                                            uri: ev.from.clone(),
                                            name: ev.from.split('/').next_back()
                                                .unwrap_or("").to_string(),
                                            size: 0,
                                            mtime_ms: 0,
                                            hash: None,
                                            is_dir: false,
                                            file_id: Some(ev.file_id.clone()),
                                            path: ev.from.clone(),
                                            created_at_ms: 0,
                                        }))
                                    }
                                    "delete" => {
                                        Some(RemoteFileEvent::Deleted {
                                            uri: ev.from.clone(),
                                            name: ev.from.split('/').next_back()
                                                .unwrap_or("").to_string(),
                                        })
                                    }
                                    _ => None,
                                };

                                if let Some(event) = remote_event {
                                    if tx.send(event).await.is_err() {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    } else if event_type == "reconnect-required" {
                        tracing::info!("SSE 服务端要求重连");
                        return Ok(());
                    }

                    event_type.clear();
                    data_buffer.clear();
                }
            }
        }

        Ok(())
    }
}

/// SSE 事件中的文件变更条目
#[derive(Debug, serde::Deserialize)]
struct SseFileEvent {
    #[serde(rename = "type")]
    event_type: String,
    file_id: String,
    from: String,
    #[serde(default)]
    #[allow(dead_code)]
    to: String,
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

/// 收集时间窗口内的批量远程事件
/// 在 run_continuous() 中使用，将窗口内的事件合并为一个 SyncPlan
pub async fn batch_remote_events(
    rx: &mut mpsc::Receiver<RemoteFileEvent>,
    window: Duration,
) -> Vec<RemoteFileEvent> {
    let mut events = Vec::new();

    // 等待第一个事件
    match tokio::time::timeout(window, rx.recv()).await {
        Ok(Some(event)) => events.push(event),
        _ => return events,
    }

    // 收集窗口内的剩余事件
    let deadline = Instant::now() + window;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(event)) => events.push(event),
            _ => break,
        }
    }

    events
}
