use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("网络错误: {0}")]
    Network(String),

    #[error("磁盘空间不足: 需要 {needed} 字节, 可用 {available} 字节")]
    DiskFull { needed: u64, available: u64 },

    #[error("认证错误: {0}")]
    Auth(String),

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
        SyncError::Network(e.to_string())
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

pub type Result<T> = std::result::Result<T, SyncError>;
