/// FFI 错误类型
#[derive(Debug, Clone)]
pub enum SyncErrorFfi {
    NotInitialized,
    NetworkError { message: String },
    DiskFull { needed: u64, available: u64 },
    AuthError { message: String },
    ConflictError { count: u32 },
    InternalError { message: String },
}

/// 同步配置
#[derive(Debug, Clone)]
pub struct SyncConfigFfi {
    pub base_url: String,
    pub access_token: String,
    pub local_root: String,
    pub remote_root: String,
    pub sync_mode: String,
    pub conflict_strategy: String,
    pub max_concurrent_transfers: u32,
    pub bandwidth_limit_kbps: u64,
    pub excluded_paths: Vec<String>,
    pub selective_dirs: Vec<String>,
}

/// 同步状态快照
#[derive(Debug, Clone)]
pub struct SyncStatusFfi {
    pub state: String,
    pub synced_files: u64,
    pub total_files: u64,
    pub uploading_count: u32,
    pub downloading_count: u32,
    pub conflict_count: u32,
    pub error_count: u32,
    pub last_sync_time: Option<String>,
    pub error_message: Option<String>,
}

/// 初始同步摘要
#[derive(Debug, Clone)]
pub struct SyncSummaryFfi {
    pub uploaded: u32,
    pub downloaded: u32,
    pub conflicts: u32,
    pub skipped: u32,
    pub deleted_local: u32,
    pub deleted_remote: u32,
    pub duration_ms: u64,
}

/// 同步事件（Rust → Dart 推送）
#[derive(Debug, Clone)]
pub enum SyncEventFfi {
    StateChanged { new_state: String },
    Progress {
        synced: u64,
        total: u64,
        current_file: String,
    },
    FileUploaded {
        local_path: String,
        remote_uri: String,
    },
    FileDownloaded {
        local_path: String,
        remote_uri: String,
    },
    ConflictDetected {
        local_path: String,
        conflict_type: String,
    },
    Error {
        message: String,
        recoverable: bool,
    },
    TokenExpired,
    DiskSpaceWarning { available_mb: u64 },
    InitialSyncComplete { summary: SyncSummaryFfi },
}

/// Android: 云端相册目录检查结果
#[derive(Debug, Clone)]
pub struct CloudAlbumCheckResultFfi {
    pub dcim_exists: bool,
    pub pictures_exists: bool,
    pub dcim_uri: Option<String>,
    pub pictures_uri: Option<String>,
}
