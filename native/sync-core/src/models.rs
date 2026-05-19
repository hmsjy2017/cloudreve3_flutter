use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ===== 同步状态 =====

#[derive(Debug, Clone, PartialEq)]
pub enum SyncState {
    Idle,
    Initializing,
    InitialSync { progress: InitialSyncProgress },
    Continuous,
    Paused,
    Error { message: String },
    Stopped,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InitialSyncProgress {
    pub scanned_local: u64,
    pub scanned_remote: u64,
    pub uploaded: u64,
    pub downloaded: u64,
    pub conflicts: u64,
    pub total_to_sync: u64,
}

// ===== 同步配置 =====

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub base_url: String,
    pub access_token: String,
    pub refresh_token: String,
    pub local_root: PathBuf,
    pub remote_root: String,
    pub sync_mode: SyncMode,
    pub conflict_strategy: ConflictStrategy,
    pub max_concurrent_transfers: usize,
    pub bandwidth_limit: Option<u64>,
    pub excluded_paths: Vec<String>,
    pub selective_dirs: Vec<String>,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncMode {
    Full,
    Selective,
    Album,
}

// ===== 文件条目 =====

#[derive(Debug, Clone)]
pub struct LocalFileEntry {
    pub relative_path: PathBuf,
    pub size: u64,
    pub mtime_ms: i64,
    pub quick_hash: String,
    pub is_dir: bool,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileEntry {
    pub uri: String,
    pub name: String,
    pub size: u64,
    pub mtime_ms: i64,
    pub hash: Option<String>,
    pub is_dir: bool,
    pub file_id: Option<String>,
    pub path: String,
    pub created_at_ms: i64,
}

// ===== 文件映射 =====

#[derive(Debug, Clone)]
pub struct FileMapping {
    pub id: i64,
    pub sync_root_id: String,
    pub local_path: PathBuf,
    pub remote_uri: String,
    pub remote_file_id: Option<String>,
    pub local_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub local_mtime: Option<i64>,
    pub remote_mtime: Option<i64>,
    pub local_size: Option<u64>,
    pub remote_size: Option<u64>,
    pub sync_status: SyncFileStatus,
    pub is_placeholder: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncFileStatus {
    Synced,
    Uploading,
    Downloading,
    Conflict,
    Placeholder,
}

// ===== 冲突 =====

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    BothModified,
    DeleteVsModify,
    NameCollision,
    TypeMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictStrategy {
    KeepLocal,
    KeepRemote,
    KeepBoth,
    NewestWins,
    LargestWins,
    Manual,
}

#[derive(Debug, Clone)]
pub enum ConflictResolution {
    UploadLocal,
    DownloadRemote,
    RenameLocal { new_name: String },
    DeleteLocal,
    DeleteRemote,
    MarkManual,
}

// ===== 传输 =====

#[derive(Debug, Clone)]
pub struct TransferTask {
    pub id: i64,
    pub sync_root_id: String,
    pub file_mapping_id: Option<i64>,
    pub direction: TransferDirection,
    pub local_path: PathBuf,
    pub remote_uri: String,
    pub file_size: u64,
    pub bytes_done: u64,
    pub status: TransferStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
    pub session_id: Option<String>,
    pub chunk_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct TransferConfig {
    pub max_concurrent: usize,
    pub max_retries: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub bandwidth_limit: Option<u64>,
    pub disk_space_reserve: u64,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            max_retries: 5,
            retry_base_delay_ms: 1000,
            retry_max_delay_ms: 60000,
            bandwidth_limit: None,
            disk_space_reserve: 1024 * 1024 * 1024, // 1GB
        }
    }
}

// ===== 同步摘要 =====

#[derive(Debug, Clone, Default)]
pub struct SyncSummary {
    pub uploaded: u32,
    pub downloaded: u32,
    pub conflicts: u32,
    pub skipped: u32,
    pub deleted_local: u32,
    pub deleted_remote: u32,
    pub duration_ms: u64,
}

// ===== 同步状态快照 =====

#[derive(Debug, Clone)]
pub struct SyncStatusSnapshot {
    pub state: SyncState,
    pub synced_files: u64,
    pub total_files: u64,
    pub uploading_count: u32,
    pub downloading_count: u32,
    pub conflict_count: u32,
    pub error_count: u32,
    pub last_sync_time: Option<String>,
    pub error_message: Option<String>,
}

// ===== 本地文件事件 =====

#[derive(Debug, Clone)]
pub enum LocalFileEvent {
    Created(Vec<PathBuf>),
    Modified(Vec<PathBuf>),
    Deleted(Vec<PathBuf>),
}

impl LocalFileEvent {
    pub fn paths(&self) -> &[PathBuf] {
        match self {
            LocalFileEvent::Created(p) => p,
            LocalFileEvent::Modified(p) => p,
            LocalFileEvent::Deleted(p) => p,
        }
    }
}

// ===== 远程文件事件 =====

#[derive(Debug, Clone)]
pub enum RemoteFileEvent {
    Created(RemoteFileEntry),
    Modified(RemoteFileEntry),
    Deleted { uri: String, name: String },
}

// ===== 平台回调事件 (Windows CFApi) =====

#[derive(Debug, Clone)]
pub enum PlatformCallbackEvent {
    HydrateRequested {
        local_path: String,
        transfer_key: i64,
    },
}

// ===== 上传会话 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSession {
    pub session_id: String,
    pub chunk_size: u64,
}

// ===== API 分页响应 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesResponse {
    pub files: Vec<RemoteFileEntry>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub next_page_token: Option<String>,
    pub is_cursor: bool,
    pub total: Option<u64>,
}

// ===== 同步计划 =====

#[derive(Debug, Clone, Default)]
pub struct SyncPlan {
    pub uploads: Vec<SyncAction>,
    pub downloads: Vec<SyncAction>,
    pub delete_local: Vec<SyncAction>,
    pub delete_remote: Vec<SyncAction>,
    pub conflicts: Vec<SyncConflict>,
    pub mkdirs_local: Vec<String>,
    pub mkdirs_remote: Vec<String>,
    /// 需要递归扫描的顶层目录（持续同步场景，目录整体提交给 Worker）
    pub scan_dirs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SyncAction {
    pub relative_path: String,
    pub local_entry: Option<LocalFileEntry>,
    pub remote_entry: Option<RemoteFileEntry>,
    pub db_mapping: Option<FileMapping>,
}

#[derive(Debug, Clone)]
pub struct SyncConflict {
    pub relative_path: String,
    pub conflict_type: ConflictType,
    pub local_entry: Option<LocalFileEntry>,
    pub remote_entry: Option<RemoteFileEntry>,
    pub db_mapping: Option<FileMapping>,
}

impl SyncPlan {
    pub fn total_actions(&self) -> u64 {
        self.uploads.len() as u64
            + self.downloads.len() as u64
            + self.delete_local.len() as u64
            + self.delete_remote.len() as u64
    }
}

// ===== Worker / Task 类型 =====

/// Worker 配置快照 — 创建时一次性读取，运行中不读任何锁
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub local_root: PathBuf,
    pub remote_root: String,
    pub max_concurrent_transfers: usize,
    pub bandwidth_limit: Option<u64>,
    pub conflict_strategy: ConflictStrategy,
    pub sync_root_id: String,
}

/// Worker 触发来源
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerTrigger {
    InitialSync,
    Continuous,
    Manual,
}

impl WorkerTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerTrigger::InitialSync => "initial_sync",
            WorkerTrigger::Continuous => "continuous",
            WorkerTrigger::Manual => "manual",
        }
    }
}

/// Worker 级别状态
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl WorkerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerStatus::Pending => "pending",
            WorkerStatus::Running => "running",
            WorkerStatus::Completed => "completed",
            WorkerStatus::Failed => "failed",
            WorkerStatus::Cancelled => "cancelled",
        }
    }

}

impl std::str::FromStr for WorkerStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(WorkerStatus::Pending),
            "running" => Ok(WorkerStatus::Running),
            "completed" => Ok(WorkerStatus::Completed),
            "failed" => Ok(WorkerStatus::Failed),
            "cancelled" => Ok(WorkerStatus::Cancelled),
            _ => Err(()),
        }
    }
}

/// 单文件操作状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskItemStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl TaskItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskItemStatus::Pending => "pending",
            TaskItemStatus::Running => "running",
            TaskItemStatus::Completed => "completed",
            TaskItemStatus::Failed => "failed",
            TaskItemStatus::Skipped => "skipped",
        }
    }

}

impl std::str::FromStr for TaskItemStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TaskItemStatus::Pending),
            "running" => Ok(TaskItemStatus::Running),
            "completed" => Ok(TaskItemStatus::Completed),
            "failed" => Ok(TaskItemStatus::Failed),
            "skipped" => Ok(TaskItemStatus::Skipped),
            _ => Err(()),
        }
    }
}

/// 操作类型 — 精确记录每个文件的操作
#[derive(Debug, Clone, PartialEq)]
pub enum TaskActionType {
    Upload,
    Download,
    DeleteLocal,
    DeleteRemote,
    Rename,
    MkdirRemote,
    MkdirLocal,
    ConflictResolve,
}

impl TaskActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskActionType::Upload => "upload",
            TaskActionType::Download => "download",
            TaskActionType::DeleteLocal => "delete_local",
            TaskActionType::DeleteRemote => "delete_remote",
            TaskActionType::Rename => "rename",
            TaskActionType::MkdirRemote => "mkdir_remote",
            TaskActionType::MkdirLocal => "mkdir_local",
            TaskActionType::ConflictResolve => "conflict_resolve",
        }
    }

}

impl std::str::FromStr for TaskActionType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "upload" => Ok(TaskActionType::Upload),
            "download" => Ok(TaskActionType::Download),
            "delete_local" => Ok(TaskActionType::DeleteLocal),
            "delete_remote" => Ok(TaskActionType::DeleteRemote),
            "rename" => Ok(TaskActionType::Rename),
            "mkdir_remote" => Ok(TaskActionType::MkdirRemote),
            "mkdir_local" => Ok(TaskActionType::MkdirLocal),
            "conflict_resolve" => Ok(TaskActionType::ConflictResolve),
            _ => Err(()),
        }
    }
}

/// Worker 级别的任务摘要 — DB 持久化，供 UI 查询
#[derive(Debug, Clone)]
pub struct SyncTask {
    pub id: String,
    pub trigger: WorkerTrigger,
    pub total_count: u32,
    pub completed_count: u32,
    pub failed_count: u32,
    pub status: WorkerStatus,
    pub created_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
}

/// 单文件操作记录 — DB 持久化，支持多维度查询
#[derive(Debug, Clone)]
pub struct SyncTaskItem {
    pub id: i64,
    pub task_id: String,
    pub relative_path: String,
    pub action_type: TaskActionType,
    pub status: TaskItemStatus,
    pub file_size: u64,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 任务项查询过滤器
#[derive(Debug, Clone, Default)]
pub struct TaskItemFilter {
    pub task_id: Option<String>,
    pub relative_path_contains: Option<String>,
    pub action_type: Option<String>,
    pub status: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

// ===== 云端相册检查结果 =====

#[derive(Debug, Clone)]
pub struct CloudAlbumCheckResult {
    pub dcim_exists: bool,
    pub pictures_exists: bool,
    pub dcim_uri: Option<String>,
    pub pictures_uri: Option<String>,
}
