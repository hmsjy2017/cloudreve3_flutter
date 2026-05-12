use crate::errors::{Result, SyncError};
use crate::models::*;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
struct ApiResponse<T> {
    code: i32,
    data: Option<T>,
    msg: Option<String>,
}

pub struct ApiClient {
    base_url: String,
    access_token: RwLock<String>,
    client: Client,
}

impl ApiClient {
    pub fn new(base_url: &str, access_token: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            access_token: RwLock::new(access_token.to_string()),
            client,
        }
    }

    pub async fn update_token(&self, token: String) {
        *self.access_token.write().await = token;
    }

    async fn token(&self) -> String {
        self.access_token.read().await.clone()
    }

    // ===== 文件列表 =====

    pub async fn list_all_files(&self, uri: &str) -> Result<Vec<RemoteFileEntry>> {
        let mut all_files = Vec::new();
        let mut next_page_token: Option<String> = None;
        let mut page = 0u32;
        let page_size = 2000u32;

        loop {
            let resp = self.list_files_page(uri, page, page_size, next_page_token.as_deref()).await?;
            let count = resp.files.len();
            all_files.extend(resp.files);

            if let Some(token) = resp.pagination.next_page_token {
                next_page_token = Some(token);
            } else if count < page_size as usize {
                break;
            } else {
                page += 1;
            }

            if all_files.len() > 1_000_000 {
                tracing::warn!("文件数量超过 100 万，截断初始同步");
                break;
            }
        }

        Ok(all_files)
    }

    pub async fn list_files_page(
        &self,
        uri: &str,
        page: u32,
        page_size: u32,
        next_page_token: Option<&str>,
    ) -> Result<ListFilesResponse> {
        let token = self.token().await;
        let mut req = self.client
            .get(format!("{}/file", self.base_url))
            .bearer_auth(&token)
            .query(&[("uri", uri), ("page", &page.to_string()), ("page_size", &page_size.to_string())]);

        if let Some(npt) = next_page_token {
            req = req.query(&[("next_page_token", npt)]);
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth("Token 过期".to_string()));
        }

        let api_resp: ApiResponse<serde_json::Value> = resp.json().await?;
        if api_resp.code != 0 {
            return Err(SyncError::Network(
                api_resp.msg.unwrap_or_else(|| "未知错误".to_string()),
            ));
        }

        let data = api_resp.data.unwrap_or_default();
        let files: Vec<RemoteFileEntry> = if let Some(objects) = data.get("objects").and_then(|o| o.as_array()) {
            objects.iter().filter_map(|obj| {
                let name = obj.get("name")?.as_str()?.to_string();
                let uri = obj.get("uri")?.as_str()?.to_string();
                let size = obj.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                let is_dir = obj.get("type").and_then(|t| t.as_u64()).unwrap_or(0) == 1;
                let hash = obj.get("hash").and_then(|h| h.as_str()).map(String::from);
                let file_id = obj.get("id").and_then(|i| i.as_str()).map(String::from);

                Some(RemoteFileEntry {
                    uri,
                    name,
                    size,
                    mtime_ms: 0,
                    hash,
                    is_dir,
                    file_id,
                })
            }).collect()
        } else {
            Vec::new()
        };

        let next_token = data.get("pagination")
            .and_then(|p| p.get("next_page_token"))
            .and_then(|t| t.as_str())
            .map(String::from);

        let total = data.get("pagination")
            .and_then(|p| p.get("total"))
            .and_then(|t| t.as_u64());

        Ok(ListFilesResponse {
            files,
            pagination: Pagination {
                next_page_token: next_token,
                total,
            },
        })
    }

    // ===== 上传 =====

    pub async fn create_upload_session(&self, uri: &str, size: u64) -> Result<UploadSession> {
        let token = self.token().await;
        let resp = self.client
            .put(format!("{}/file/upload", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uri": uri,
                "size": size,
            }))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth("Token 过期".to_string()));
        }

        let api_resp: ApiResponse<serde_json::Value> = resp.json().await?;
        if api_resp.code != 0 {
            return Err(SyncError::Network(
                api_resp.msg.unwrap_or_else(|| "创建上传会话失败".to_string()),
            ));
        }

        let data = api_resp.data.unwrap_or_default();
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
        let token = self.token().await;
        let resp = self.client
            .post(format!(
                "{}/file/upload/{}/{}",
                self.base_url, session_id, index
            ))
            .bearer_auth(&token)
            .header("Content-Length", data.len())
            .body(data.to_vec())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SyncError::Network(format!(
                "上传分片失败: HTTP {}",
                resp.status()
            )));
        }

        Ok(())
    }

    // ===== 下载 =====

    pub async fn get_download_url(&self, uris: &[&str]) -> Result<Vec<String>> {
        let token = self.token().await;
        let resp = self.client
            .post(format!("{}/file/url", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uris": uris,
                "download": false,
            }))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth("Token 过期".to_string()));
        }

        let api_resp: ApiResponse<serde_json::Value> = resp.json().await?;
        if api_resp.code != 0 {
            return Err(SyncError::Network(
                api_resp.msg.unwrap_or_else(|| "获取下载 URL 失败".to_string()),
            ));
        }

        let data = api_resp.data.unwrap_or_default();
        let urls = if let Some(arr) = data.as_array() {
            arr.iter()
                .filter_map(|item| item.get("url")?.as_str().map(String::from))
                .collect()
        } else {
            Vec::new()
        };

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

    // ===== 目录操作 =====

    pub async fn create_directory(&self, parent_uri: &str, name: &str) -> Result<RemoteFileEntry> {
        let token = self.token().await;
        let resp = self.client
            .put(format!("{}/file", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uri": parent_uri,
                "name": name,
                "type": 1,
            }))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth("Token 过期".to_string()));
        }

        let api_resp: ApiResponse<serde_json::Value> = resp.json().await?;
        if api_resp.code != 0 {
            return Err(SyncError::Network(
                api_resp.msg.unwrap_or_else(|| "创建目录失败".to_string()),
            ));
        }

        Ok(RemoteFileEntry {
            uri: format!("{}/{}", parent_uri, name),
            name: name.to_string(),
            size: 0,
            mtime_ms: 0,
            hash: None,
            is_dir: true,
            file_id: None,
        })
    }

    // ===== 删除 =====

    pub async fn delete_files(&self, uris: &[&str]) -> Result<()> {
        let token = self.token().await;
        let resp = self.client
            .delete(format!("{}/file", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uris": uris,
            }))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth("Token 过期".to_string()));
        }

        Ok(())
    }

    // ===== SSE 事件流 (Phase 3 将实现) =====

    pub async fn connect_sse(&self, _client_id: &str) -> Result<SseEventStream> {
        // SSE 连接将在 Phase 3 完整实现
        // 目前返回一个空流
        Ok(SseEventStream)
    }
}

/// SSE 事件流占位
pub struct SseEventStream;

#[derive(Debug, Clone)]
pub enum SseEvent {
    Resumed,
    FileChange(Vec<RemoteFileEvent>),
    ReconnectRequired,
}
