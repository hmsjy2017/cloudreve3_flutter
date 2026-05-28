use std::error::Error as StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("网络错误: {0}")]
    Network(String),

    #[error("磁盘空间不足: 需要 {needed} 字节, 可用 {available} 字节")]
    DiskFull { needed: u64, available: u64 },

    #[error("认证错误: {0}")]
    Auth(String),

    #[error("远程文件已存在")]
    ObjectExisted,

    #[error("存储策略不允许: {0}")]
    StoragePolicyDenied(String),

    #[error("上传失败: {0}")]
    UploadFailed(String),

    #[error("文件未找到: {0}")]
    FileNotFound(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("文件锁定冲突")]
    LockConflict { tokens: Vec<LockConflictItem> },

    #[error("冲突: {count} 个文件存在冲突")]
    Conflict { count: u32 },

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("文件系统错误: {0}")]
    FileSystem(String),

    #[error("路径遍历攻击: {path} 不在 {root} 下")]
    PathTraversal { path: String, root: String },

    #[error("引擎未初始化")]
    NotInitialized,

    #[error("引擎已初始化")]
    AlreadyInitialized,

    #[error("同步已取消")]
    Cancelled,

    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<rusqlite::Error> for SyncError {
    fn from(e: rusqlite::Error) -> Self {
        SyncError::Database(e.to_string())
    }
}

impl From<r2d2::Error> for SyncError {
    fn from(e: r2d2::Error) -> Self {
        SyncError::Database(e.to_string())
    }
}

impl From<reqwest::Error> for SyncError {
    fn from(e: reqwest::Error) -> Self {
        let mut detail = String::new();
        if e.is_connect() {
            detail.push_str("连接失败");
        } else if e.is_timeout() {
            detail.push_str("请求超时");
        } else if e.is_request() {
            detail.push_str("请求构建失败");
        } else if e.is_body() {
            detail.push_str("请求体错误");
        } else if e.is_decode() {
            detail.push_str("响应解码失败");
        } else if e.is_redirect() {
            detail.push_str("重定向过多");
        }
        let url = e.url().map(|u| u.to_string()).unwrap_or_default();
        let source = StdError::source(&e)
            .map(|s| format!(": {s}"))
            .unwrap_or_default();
        let msg = e.to_string();
        // 如果 detail 为空，用 reqwest 原始消息
        if detail.is_empty() {
            SyncError::Network(if url.is_empty() {
                format!("{msg}{source}")
            } else {
                format!("{msg} [{url}]{source}")
            })
        } else {
            SyncError::Network(if url.is_empty() {
                format!("{detail}: {msg}{source}")
            } else {
                format!("{detail}: {msg} [{url}]{source}")
            })
        }
    }
}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        SyncError::FileSystem(e.to_string())
    }
}

impl From<tokio::task::JoinError> for SyncError {
    fn from(e: tokio::task::JoinError) -> Self {
        SyncError::Internal(e.to_string())
    }
}

/// 锁冲突条目 — 来自 40073 响应的 data 数组
#[derive(Debug, Clone)]
pub struct LockConflictItem {
    pub path: String,
    pub token: String,
}

pub type Result<T> = std::result::Result<T, SyncError>;
