use crate::api_client::ApiClient;
use crate::errors::Result;
use crate::models::{RemoteFileEntry, RemoteFileEvent};
use eventsource::event::{parse_event_line, Event, ParseResult};
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

        let api = self.api.clone();
        let client_id = self.client_id.clone();
        let remote_root = uri.to_string();

        tokio::spawn(async move {
            loop {
                let token = api.token().await;
                let base_url = api.base_url().to_string();

                match Self::connect_sse(&base_url, &token, &client_id, &remote_root, &tx).await {
                    Ok(_) => {
                        tracing::info!("[SSE] 连接关闭，5秒后重连...");
                    }
                    Err(e) => {
                        tracing::warn!("[SSE] 连接错误: {}，5秒后重连...", e);
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;

                if tx.is_closed() {
                    break;
                }
            }

            tracing::info!("[SSE] 订阅任务退出");
        });

        Ok(rx)
    }

    /// 建立 SSE 连接，使用 eventsource parser 正确解析事件
    async fn connect_sse(
        base_url: &str,
        token: &str,
        client_id: &str,
        remote_root: &str,
        tx: &mpsc::Sender<RemoteFileEvent>,
    ) -> Result<()> {
        let url = format!("{}/file/events", base_url);

        tracing::info!("[SSE] 正在连接: {}?uri={}", url, remote_root);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .bearer_auth(token)
            .header("X-Cr-Client-Id", client_id)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .query(&[("uri", remote_root)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(crate::errors::SyncError::Network(
                format!("[SSE] 连接失败: HTTP {}", resp.status()),
            ));
        }

        tracing::info!("[SSE] 连接成功，开始监听事件 (uri={})", remote_root);

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;

        let mut line_buf = String::new();
        let mut event = Event::new();
        let mut event_count: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| crate::errors::SyncError::Network(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);
            line_buf.push_str(&text);

            while let Some(newline_pos) = line_buf.find('\n') {
                let line = line_buf[..=newline_pos].to_string();
                line_buf = line_buf[newline_pos + 1..].to_string();

                match parse_event_line(&line, &mut event) {
                    ParseResult::Next => {}
                    ParseResult::Dispatch => {
                        if event.is_empty() {
                            event.clear();
                            continue;
                        }

                        event_count += 1;
                        let event_type = event.event_type.clone().unwrap_or_default();

                        if event_type == "event" {
                            let data = event.data.trim();
                            let events: Vec<SseFileEvent> = if data.starts_with('[') {
                                serde_json::from_str(data).unwrap_or_default()
                            } else {
                                serde_json::from_str(data)
                                    .map(|e| vec![e])
                                    .unwrap_or_else(|_| {
                                        tracing::warn!("[SSE] 无法解析事件数据: {}", data);
                                        Vec::new()
                                    })
                            };

                            for ev in &events {
                                tracing::info!(
                                    "[SSE] 原始事件: type={}, file_id={}, from={}, to={}",
                                    ev.event_type, ev.file_id, ev.from, ev.to
                                );
                                if let Some(remote_event) = Self::parse_sse_event(ev, remote_root) {
                                    if tx.send(remote_event).await.is_err() {
                                        return Ok(());
                                    }
                                }
                            }
                        } else if event_type == "reconnect-required" {
                            tracing::info!("[SSE] 服务端要求重连");
                            return Ok(());
                        } else if event_type == "subscribed" {
                            tracing::info!("[SSE] 订阅确认成功");
                        } else if event_type == "keep-alive" {
                        } else {
                            tracing::debug!(
                                "[SSE] 忽略未知事件: type={:?}, data={}",
                                event.event_type,
                                event.data.trim()
                            );
                        }

                        event.clear();
                    }
                    ParseResult::SetRetry(retry) => {
                        tracing::debug!("[SSE] 服务端设置重试间隔: {:?}", retry);
                    }
                }
            }
        }

        tracing::info!("[SSE] 流结束，共处理 {} 个事件", event_count);
        Ok(())
    }

    /// 将 SSE 文件事件转为内部 RemoteFileEvent
    /// `remote_root` 用于将 SSE 的相对路径 `from`/`to` 拼接为完整 URI
    ///
    /// SSE rename 事件需要区分两种情况：
    /// - 真重命名：from 和 to 的父目录相同，仅文件名不同
    ///   例: from=/Books/log.txt.aa, to=/Books/log.txt.aaa
    /// - 移动：from 和 to 的父目录不同
    ///   例: from=/Readest/log.txt.aa, to=/Readest/Books/log.txt.aa
    fn parse_sse_event(ev: &SseFileEvent, remote_root: &str) -> Option<RemoteFileEvent> {
        let full_uri = |path: &str| -> String {
            if path.starts_with("cloudreve://") || path.starts_with("http") {
                path.to_string()
            } else {
                format!("{}{}", remote_root.trim_end_matches('/'), path)
            }
        };

        match ev.event_type.as_str() {
            "create" | "modify" => {
                let uri = full_uri(&ev.from);
                Some(RemoteFileEvent::Modified(RemoteFileEntry {
                    name: ev.from.split('/').next_back()
                        .unwrap_or("").to_string(),
                    path: ev.from.clone(),
                    uri,
                    size: 0,
                    mtime_ms: 0,
                    hash: None,
                    is_dir: false,
                    file_id: Some(ev.file_id.clone()),
                    created_at_ms: 0,
                }))
            }
            "delete" => {
                let uri = full_uri(&ev.from);
                Some(RemoteFileEvent::Deleted {
                    uri,
                    name: ev.from.split('/').next_back()
                        .unwrap_or("").to_string(),
                })
            }
            "rename" | "move" => {
                let old_uri = full_uri(&ev.from);
                let new_uri = full_uri(&ev.to);
                let new_entry = RemoteFileEntry {
                    name: ev.to.split('/').next_back()
                        .unwrap_or("").to_string(),
                    path: ev.to.clone(),
                    uri: new_uri,
                    size: 0,
                    mtime_ms: 0,
                    hash: None,
                    is_dir: false,
                    file_id: Some(ev.file_id.clone()),
                    created_at_ms: 0,
                };

                // 判断是真 rename 还是 move：比较 from 和 to 的父目录
                let from_parent = ev.from.rfind('/').map(|i| &ev.from[..i]).unwrap_or("");
                let to_parent = ev.to.rfind('/').map(|i| &ev.to[..i]).unwrap_or("");

                if from_parent == to_parent {
                    Some(RemoteFileEvent::Renamed { old_uri, new_entry })
                } else {
                    Some(RemoteFileEvent::Moved { old_uri, new_entry })
                }
            }
            _ => {
                tracing::debug!("[SSE] 忽略未知事件类型: {}", ev.event_type);
                None
            }
        }
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

    pub fn cleanup(&mut self) {
        let now = Instant::now();
        self.pending.retain(|_, last| now.duration_since(*last) < self.debounce_window);
    }
}

/// 收集时间窗口内的批量远程事件
pub async fn batch_remote_events(
    rx: &mut mpsc::Receiver<RemoteFileEvent>,
    window: Duration,
) -> Vec<RemoteFileEvent> {
    let mut events = Vec::new();

    match tokio::time::timeout(window, rx.recv()).await {
        Ok(Some(event)) => events.push(event),
        _ => return events,
    }

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
