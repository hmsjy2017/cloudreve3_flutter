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

    /// 检查 API 响应状态，提取 data 或返回错误
    async fn check_response(&self, resp: reqwest::Response) -> Result<serde_json::Value> {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth("Token 过期".to_string()));
        }
        if !resp.status().is_success() {
            return Err(SyncError::Network(format!("HTTP {}", resp.status())));
        }
        let api_resp: ApiResponse<serde_json::Value> = resp.json().await?;
        if api_resp.code != 0 {
            return Err(SyncError::Network(
                api_resp.msg.unwrap_or_else(|| "未知错误".to_string()),
            ));
        }
        Ok(api_resp.data.unwrap_or_default())
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
            .query(&[
                ("uri", uri),
                ("page", &page.to_string()),
                ("page_size", &page_size.to_string()),
            ]);

        if let Some(npt) = next_page_token {
            req = req.query(&[("next_page_token", npt)]);
        }

        let resp = req.send().await?;
        let data = self.check_response(resp).await?;

        // 解析文件列表: data.objects 数组
        let files: Vec<RemoteFileEntry> = if let Some(objects) = data.get("objects").and_then(|o| o.as_array()) {
            objects.iter().filter_map(|obj| {
                let name = obj.get("name")?.as_str()?.to_string();
                let uri = obj.get("uri").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let path = obj.get("path")?.as_str().unwrap_or("").to_string();
                let size = obj.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                let file_type = obj.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
                let is_dir = file_type == 1;
                let file_id = obj.get("id")?.as_str()?.to_string();
                let created_at = obj.get("created_at").and_then(|t| t.as_str()).unwrap_or("");
                let updated_at = obj.get("updated_at").and_then(|t| t.as_str()).unwrap_or("");

                Some(RemoteFileEntry {
                    uri,
                    name,
                    size,
                    mtime_ms: parse_timestamp(updated_at),
                    hash: None,
                    is_dir,
                    file_id: Some(file_id),
                    path,
                    created_at_ms: parse_timestamp(created_at),
                })
            }).collect()
        } else {
            Vec::new()
        };

        // 解析分页信息
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

        let data = self.check_response(resp).await?;

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

        let data = self.check_response(resp).await?;

        // data.urls 数组
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
        let token = self.token().await;
        let uri = format!("{}/{}", parent_uri, name);
        let resp = self.client
            .post(format!("{}/file/create", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uri": uri,
                "type": "folder",
            }))
            .send()
            .await?;

        let _data = self.check_response(resp).await?;

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
        let token = self.token().await;
        let resp = self.client
            .delete(format!("{}/file", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uris": uris,
            }))
            .send()
            .await?;

        self.check_response(resp).await?;
        Ok(())
    }

    // ===== 移动 =====

    pub async fn move_files(&self, src_uris: &[&str], dst_uri: &str, copy: bool) -> Result<()> {
        let token = self.token().await;
        let resp = self.client
            .post(format!("{}/file/move", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uris": src_uris,
                "dst": dst_uri,
                "copy": copy,
            }))
            .send()
            .await?;

        self.check_response(resp).await?;
        Ok(())
    }

    // ===== 重命名 =====

    pub async fn rename_file(&self, uri: &str, new_name: &str) -> Result<()> {
        let token = self.token().await;
        let resp = self.client
            .post(format!("{}/file/rename", self.base_url))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "uri": uri,
                "new_name": new_name,
            }))
            .send()
            .await?;

        self.check_response(resp).await?;
        Ok(())
    }

    // ===== 获取文件信息 =====

    pub async fn get_file_info(&self, uri: &str) -> Result<RemoteFileEntry> {
        let token = self.token().await;
        let resp = self.client
            .get(format!("{}/file/info", self.base_url))
            .bearer_auth(&token)
            .query(&[("uri", uri)])
            .send()
            .await?;

        let data = self.check_response(resp).await?;

        Ok(RemoteFileEntry {
            uri: data.get("uri").and_then(|u| u.as_str()).unwrap_or(uri).to_string(),
            name: data.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
            size: data.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
            mtime_ms: data.get("updated_at").and_then(|t| t.as_str()).map(parse_timestamp).unwrap_or(0),
            hash: None,
            is_dir: data.get("type").and_then(|t| t.as_u64()).unwrap_or(0) == 1,
            file_id: data.get("id").and_then(|i| i.as_str()).map(String::from),
            path: data.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
            created_at_ms: data.get("created_at").and_then(|t| t.as_str()).map(parse_timestamp).unwrap_or(0),
        })
    }
}

/// 解析 Cloudreve 时间戳 (ISO 8601 或 Unix ms)
fn parse_timestamp(s: &str) -> i64 {
    // 尝试解析 ISO 8601
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp_millis();
    }
    // 尝试解析 Unix 秒
    if let Ok(ts) = s.parse::<i64>() {
        // 如果值在合理范围内认为是秒
        if ts < 10_000_000_000 {
            return ts * 1000;
        }
        return ts; // 已经是毫秒
    }
    0
}
