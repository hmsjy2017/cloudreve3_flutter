use crate::errors::{Result, SyncError};
use crate::models::*;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Deserialize)]
struct ApiResponse<T> {
    code: i32,
    data: Option<T>,
    msg: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
}

/// Token 刷新状态：确保并发请求只刷新一次
struct RefreshState {
    refreshing: bool,
    notify: Arc<tokio::sync::Notify>,
}

pub struct ApiClient {
    base_url: String,
    access_token: RwLock<String>,
    refresh_token: RwLock<String>,
    refresh_state: Arc<Mutex<RefreshState>>,
    client: Client,
}

impl ApiClient {
    pub fn new(base_url: &str, access_token: &str, refresh_token: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            access_token: RwLock::new(access_token.to_string()),
            refresh_token: RwLock::new(refresh_token.to_string()),
            refresh_state: Arc::new(Mutex::new(RefreshState { refreshing: false, notify: Arc::new(tokio::sync::Notify::new()) })),
            client,
        }
    }

    pub async fn update_token(&self, token: String) {
        *self.access_token.write().await = token;
    }

    pub async fn update_tokens(&self, access: &str, refresh: &str) {
        *self.access_token.write().await = access.to_string();
        *self.refresh_token.write().await = refresh.to_string();
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn token(&self) -> String {
        self.access_token.read().await.clone()
    }

    /// 带并发去重的 token 刷新
    /// 多个任务同时遇到 401 时，只有一个执行刷新，其他等待刷新完成后自动获取新 token
    async fn refresh_access_token(&self) -> Result<()> {
        let mut state = self.refresh_state.lock().await;

        if state.refreshing {
            // 已经有另一个任务在刷新了，等待通知
            let notify = state.notify.clone();
            drop(state);
            notify.notified().await;
            return Ok(());
        }

        state.refreshing = true;
        let notify = state.notify.clone();
        drop(state);

        let result = self.do_refresh().await;

        let mut state = self.refresh_state.lock().await;
        state.refreshing = false;
        drop(state);

        // 通知所有等待者刷新已完成
        notify.notify_waiters();

        result
    }

    async fn do_refresh(&self) -> Result<()> {
        let refresh_token = self.refresh_token.read().await.clone();
        if refresh_token.is_empty() {
            return Err(SyncError::Auth("无 refresh_token，无法刷新".into()));
        }

        tracing::info!("正在刷新 access_token...");

        let resp = self.client
            .post(format!("{}/session/token/refresh", self.base_url))
            .json(&serde_json::json!({
                "refresh_token": refresh_token,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SyncError::Auth(format!("Token 刷新失败: HTTP {}", resp.status())));
        }

        let api_resp: ApiResponse<RefreshResponse> = resp.json().await?;
        if api_resp.code != 0 {
            return Err(SyncError::Auth(format!(
                "Token 刷新失败: {}",
                api_resp.msg.unwrap_or_else(|| "未知错误".into())
            )));
        }

        if let Some(data) = api_resp.data {
            *self.access_token.write().await = data.access_token;
            *self.refresh_token.write().await = data.refresh_token;
            tracing::info!("access_token 刷新成功");
            Ok(())
        } else {
            Err(SyncError::Auth("Token 刷新响应缺少数据".into()))
        }
    }

    /// 解析 API 响应
    async fn parse_response(&self, resp: reqwest::Response) -> Result<serde_json::Value> {
        if !resp.status().is_success() {
            return Err(SyncError::Network(format!("HTTP {}", resp.status())));
        }
        let api_resp: ApiResponse<serde_json::Value> = resp.json().await?;
        if api_resp.code == 401 {
            return Err(SyncError::Auth("Login required".into()));
        }
        if api_resp.code == 40004 {
            return Err(SyncError::ObjectExisted);
        }
        if api_resp.code == 40073 {
            let items = api_resp.data
                .and_then(|d| d.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|item| {
                    let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let token = item.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if !token.is_empty() {
                        Some(crate::errors::LockConflictItem { path, token })
                    } else {
                        None
                    }
                })
                .collect();
            return Err(SyncError::LockConflict { tokens: items });
        }
        if api_resp.code != 0 {
            return Err(SyncError::Network(
                api_resp.msg.unwrap_or_else(|| format!("错误码: {}", api_resp.code)),
            ));
        }
        Ok(api_resp.data.unwrap_or_default())
    }

    /// 发送带认证的请求，自动处理 401（刷新 token 后重试一次）
    /// request_builder 接收当前 token，返回 RequestBuilder
    async fn send_with_auth_retry(
        &self,
        request_builder: impl Fn(String) -> reqwest::RequestBuilder,
    ) -> Result<serde_json::Value> {
        // 第一次尝试
        let token = self.token().await;
        let resp = request_builder(token).send().await?;
        let result = self.parse_response(resp).await;

        if let Err(SyncError::Auth(_)) = result {
            // 刷新 token
            self.refresh_access_token().await?;
            // 用新 token 重试
            let new_token = self.token().await;
            let resp = request_builder(new_token).send().await?;
            return self.parse_response(resp).await;
        }

        result
    }

    // ===== 文件列表 =====

    /// 递归列出指定 URI 下的所有文件和目录
    pub async fn list_all_files(&self, uri: &str) -> Result<Vec<RemoteFileEntry>> {
        let mut all_files = Vec::new();
        self.list_all_files_recursive(uri, &mut all_files).await?;
        Ok(all_files)
    }

    fn list_all_files_recursive<'a>(
        &'a self,
        uri: &'a str,
        result: &'a mut Vec<RemoteFileEntry>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if result.len() > 1_000_000 {
                tracing::warn!("文件数量超过 100 万，截断初始同步");
                return Ok(());
            }

            let mut next_page_token: Option<String> = None;
            let mut page = 0u32;
            let page_size = 2000u32;
            let mut dirs_to_recurse = Vec::new();

            loop {
                let resp = self.list_files_page(uri, page, page_size, next_page_token.as_deref()).await?;
                let count = resp.files.len();

                for file in resp.files {
                    if file.is_dir {
                        dirs_to_recurse.push(file.uri.clone());
                    }
                    result.push(file);
                }

                if let Some(token) = resp.pagination.next_page_token {
                    next_page_token = Some(token);
                } else if count < page_size as usize {
                    break;
                } else {
                    page += 1;
                }
            }

            for dir_uri in dirs_to_recurse {
                self.list_all_files_recursive(&dir_uri, result).await?;
            }

            Ok(())
        })
    }

    pub async fn list_files_page(
        &self,
        uri: &str,
        page: u32,
        page_size: u32,
        next_page_token: Option<&str>,
    ) -> Result<ListFilesResponse> {
        let data = self.send_with_auth_retry(|token| {
            let mut req = self.client
                .get(format!("{}/file", self.base_url))
                .bearer_auth(&token)
                .query(&[
                    ("uri", uri),
                    ("page", &page.to_string()),
                    ("page_size", &page_size.to_string()),
                ]);
            if let Some(npt) = next_page_token {
                req = req.query(&[("next_page_token", npt)]);
            }
            req
        }).await?;

        let parent_uri = uri.to_string();
        let files: Vec<RemoteFileEntry> = if let Some(items) = data.get("files").and_then(|f| f.as_array()) {
            items.iter().filter_map(|obj| {
                let name_raw = obj.get("name")?.as_str()?.to_string();
                let name = percent_decode_str(&name_raw);
                let path_raw = obj.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let path = percent_decode_str(&path_raw);
                let entry_uri = if path_raw.is_empty() {
                    format!("{}/{}", parent_uri, name_raw)
                } else {
                    path_raw.clone()
                };
                let size = obj.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                let file_type = obj.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
                let is_dir = file_type == 1;
                let file_id = obj.get("id").and_then(|s| s.as_str()).map(String::from);
                let created_at = obj.get("created_at").and_then(|t| t.as_str()).unwrap_or("");
                let updated_at = obj.get("updated_at").and_then(|t| t.as_str()).unwrap_or("");

                Some(RemoteFileEntry {
                    uri: entry_uri,
                    name,
                    size,
                    mtime_ms: parse_timestamp(updated_at),
                    hash: None,
                    is_dir,
                    file_id,
                    path,
                    created_at_ms: parse_timestamp(created_at),
                })
            }).collect()
        } else {
            tracing::warn!("API 响应中未找到 files 数组, data keys: {:?}", data.as_object().map(|m| m.keys().collect::<Vec<_>>()));
            Vec::new()
        };

        let pagination = data.get("pagination").cloned().unwrap_or_default();
        let next_token = pagination.get("next_page_token")
            .and_then(|t| t.as_str())
            .map(String::from);
        let is_cursor = pagination.get("is_cursor")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        let total = pagination.get("total_items")
            .and_then(|t| t.as_u64())
            .or_else(|| pagination.get("total").and_then(|t| t.as_u64()));

        Ok(ListFilesResponse {
            files,
            pagination: Pagination {
                next_page_token: next_token,
                is_cursor,
                total,
            },
        })
    }

    // ===== 上传 =====

    pub async fn create_upload_session(
        &self,
        uri: &str,
        size: u64,
        overwrite: bool,
        last_modified: Option<i64>,
        mime_type: Option<&str>,
        policy_id: Option<&str>,
    ) -> Result<UploadSession> {
        let mut body = serde_json::json!({
            "uri": uri,
            "size": size,
        });
        if overwrite {
            body["entity_type"] = serde_json::Value::String("version".to_string());
        }
        if let Some(mtime) = last_modified {
            body["last_modified"] = serde_json::Value::Number(mtime.into());
        }
        if let Some(mime) = mime_type {
            body["mime_type"] = serde_json::Value::String(mime.to_string());
        }
        if let Some(pid) = policy_id {
            body["policy_id"] = serde_json::Value::String(pid.to_string());
        }

        let data = self.send_with_auth_retry(|token| {
            self.client
                .put(format!("{}/file/upload", self.base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await?;

        let session_id = data.get("session_id")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let chunk_size = data.get("chunk_size")
            .and_then(|c| c.as_u64())
            .unwrap_or(10 * 1024 * 1024);

        Ok(UploadSession {
            session_id,
            chunk_size,
        })
    }

    pub async fn upload_chunk(
        &self,
        session_id: &str,
        index: u32,
        data: &[u8],
    ) -> Result<()> {
        let chunk_data = data.to_vec();
        let content_len = data.len().to_string();
        self.send_with_auth_retry(|token| {
            self.client
                .post(format!(
                    "{}/file/upload/{}/{}",
                    self.base_url, session_id, index
                ))
                .bearer_auth(&token)
                .header("Content-Length", &content_len)
                .body(chunk_data.clone())
        }).await?;
        Ok(())
    }

    // ===== 下载 =====

    pub async fn get_download_url(&self, uris: &[&str]) -> Result<Vec<String>> {
        let body = serde_json::json!({
            "uris": uris,
            "download": false,
        });

        let data = self.send_with_auth_retry(|token| {
            self.client
                .post(format!("{}/file/url", self.base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await?;

        let urls = data.get("urls")
            .and_then(|u| u.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("url")?.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(urls)
    }

    pub async fn stream_download(
        &self,
        url: &str,
        offset: u64,
    ) -> Result<reqwest::Response> {
        let mut req = self.client.get(url);
        if offset > 0 {
            req = req.header("Range", format!("bytes={}-", offset));
        }
        let resp = req.send().await?;
        Ok(resp)
    }

    // ===== 创建文件/目录 =====

    pub async fn create_directory(&self, parent_uri: &str, name: &str) -> Result<RemoteFileEntry> {
        let uri = format!("{}/{}", parent_uri, name);
        let body = serde_json::json!({
            "uri": uri,
            "type": "folder",
        });

        let _data = self.send_with_auth_retry(|token| {
            self.client
                .post(format!("{}/file/create", self.base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await?;

        Ok(RemoteFileEntry {
            uri,
            name: name.to_string(),
            size: 0,
            mtime_ms: 0,
            hash: None,
            is_dir: true,
            file_id: None,
            path: String::new(),
            created_at_ms: 0,
        })
    }

    // ===== 删除 =====

    pub async fn delete_files(&self, uris: &[&str]) -> Result<()> {
        let body = serde_json::json!({
            "uris": uris,
        });

        let result = self.send_with_auth_retry(|token| {
            let client = &self.client;
            let base_url = &self.base_url;
            let body = body.clone();
            client
                .delete(format!("{}/file", base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await;

        match result {
            Ok(_) => Ok(()),
            Err(SyncError::LockConflict { tokens }) => {
                for item in &tokens {
                    tracing::warn!(
                        "删除异常: code(40073), 进行解锁, token: {}, path: {}",
                        item.token, item.path
                    );
                }
                self.force_unlock_files(&tokens).await?;

                // 解锁后重试删除
                self.send_with_auth_retry(|token| {
                    let client = &self.client;
                    let base_url = &self.base_url;
                    let body = body.clone();
                    client
                        .delete(format!("{}/file", base_url))
                        .bearer_auth(&token)
                        .json(&body)
                }).await?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// 强制解锁文件 — DELETE /file/lock
    pub async fn force_unlock_files(&self, items: &[crate::errors::LockConflictItem]) -> Result<()> {
        let tokens: Vec<&str> = items.iter().map(|i| i.token.as_str()).collect();
        if tokens.is_empty() {
            return Ok(());
        }

        let body = serde_json::json!({
            "tokens": tokens,
        });

        self.send_with_auth_retry(|token| {
            let client = &self.client;
            let base_url = &self.base_url;
            let body = body.clone();
            client
                .delete(format!("{}/file/lock", base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await?;

        tracing::info!("强制解锁完成: {} 个文件", items.len());
        Ok(())
    }

    // ===== 移动 =====

    pub async fn move_files(&self, src_uris: &[&str], dst_uri: &str, copy: bool) -> Result<()> {
        let body = serde_json::json!({
            "uris": src_uris,
            "dst": dst_uri,
            "copy": copy,
        });

        self.send_with_auth_retry(|token| {
            self.client
                .post(format!("{}/file/move", self.base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await?;

        Ok(())
    }

    // ===== 重命名 =====

    pub async fn rename_file(&self, uri: &str, new_name: &str) -> Result<()> {
        let body = serde_json::json!({
            "uri": uri,
            "new_name": new_name,
        });

        self.send_with_auth_retry(|token| {
            self.client
                .post(format!("{}/file/rename", self.base_url))
                .bearer_auth(&token)
                .json(&body)
        }).await?;

        Ok(())
    }

    // ===== 获取文件信息 =====

    pub async fn get_file_info(&self, uri: &str) -> Result<RemoteFileEntry> {
        let data = self.send_with_auth_retry(|token| {
            self.client
                .get(format!("{}/file/info", self.base_url))
                .bearer_auth(&token)
                .query(&[("uri", uri)])
        }).await?;

        Ok(RemoteFileEntry {
            uri: data.get("uri").and_then(|u| u.as_str()).unwrap_or(uri).to_string(),
            name: percent_decode_str(data.get("name").and_then(|n| n.as_str()).unwrap_or("")),
            size: data.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
            mtime_ms: data.get("updated_at").and_then(|t| t.as_str()).map(parse_timestamp).unwrap_or(0),
            hash: None,
            is_dir: data.get("type").and_then(|t| t.as_u64()).unwrap_or(0) == 1,
            file_id: data.get("id").and_then(|i| i.as_str()).map(String::from),
            path: percent_decode_str(data.get("path").and_then(|p| p.as_str()).unwrap_or("")),
            created_at_ms: data.get("created_at").and_then(|t| t.as_str()).map(parse_timestamp).unwrap_or(0),
        })
    }
}

/// 解析 Cloudreve 时间戳 (ISO 8601 或 Unix ms)
fn parse_timestamp(s: &str) -> i64 {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp_millis();
    }
    if let Ok(ts) = s.parse::<i64>() {
        if ts < 10_000_000_000 {
            return ts * 1000;
        }
        return ts;
    }
    0
}

fn percent_decode_str(s: &str) -> String {
    urlencoding::decode(s).unwrap_or_else(|_| s.into()).to_string()
}
