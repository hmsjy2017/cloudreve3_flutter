use flutter_rust_bridge::frb;
use tokio::sync::Mutex;

use crate::api::ffi_types::*;
use crate::sync_engine::SyncEngine;

use std::sync::Arc;

/// 全局引擎实例（懒初始化）
static ENGINE: once_cell::sync::Lazy<Arc<Mutex<Option<SyncEngine>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

// 内部类型 -> FFI 类型的转换函数

fn error_to_ffi(e: crate::errors::SyncError) -> SyncErrorFfi {
    match e {
        crate::errors::SyncError::Network(msg) => SyncErrorFfi::NetworkError { message: msg },
        crate::errors::SyncError::DiskFull { needed, available } => {
            SyncErrorFfi::DiskFull { needed, available }
        }
        crate::errors::SyncError::Auth(msg) => SyncErrorFfi::AuthError { message: msg },
        crate::errors::SyncError::Conflict { count } => SyncErrorFfi::ConflictError { count },
        crate::errors::SyncError::NotInitialized => SyncErrorFfi::NotInitialized,
        _ => SyncErrorFfi::InternalError {
            message: e.to_string(),
        },
    }
}

fn config_from_ffi(ffi: SyncConfigFfi) -> crate::models::SyncConfig {
    use crate::models::{ConflictStrategy, SyncMode};
    use std::path::PathBuf;

    let sync_mode = match ffi.sync_mode.as_str() {
        "selective" => SyncMode::Selective,
        "album" => SyncMode::Album,
        _ => SyncMode::Full,
    };

    let conflict_strategy = match ffi.conflict_strategy.as_str() {
        "keep_local" => ConflictStrategy::KeepLocal,
        "keep_remote" => ConflictStrategy::KeepRemote,
        "newest_wins" => ConflictStrategy::NewestWins,
        "largest_wins" => ConflictStrategy::LargestWins,
        "manual" => ConflictStrategy::Manual,
        _ => ConflictStrategy::KeepBoth,
    };

    let bandwidth_limit = if ffi.bandwidth_limit_kbps > 0 {
        Some(ffi.bandwidth_limit_kbps * 1024)
    } else {
        None
    };

    crate::models::SyncConfig {
        base_url: ffi.base_url,
        access_token: ffi.access_token,
        local_root: PathBuf::from(&ffi.local_root),
        remote_root: ffi.remote_root,
        sync_mode,
        conflict_strategy,
        max_concurrent_transfers: ffi.max_concurrent_transfers as usize,
        bandwidth_limit,
        excluded_paths: ffi.excluded_paths,
        selective_dirs: ffi.selective_dirs,
    }
}

fn config_to_ffi(c: &crate::models::SyncConfig) -> SyncConfigFfi {
    use crate::models::{ConflictStrategy, SyncMode};

    let sync_mode = match c.sync_mode {
        SyncMode::Full => "full",
        SyncMode::Selective => "selective",
        SyncMode::Album => "album",
    };

    let conflict_strategy = match c.conflict_strategy {
        ConflictStrategy::KeepLocal => "keep_local",
        ConflictStrategy::KeepRemote => "keep_remote",
        ConflictStrategy::KeepBoth => "keep_both",
        ConflictStrategy::NewestWins => "newest_wins",
        ConflictStrategy::LargestWins => "largest_wins",
        ConflictStrategy::Manual => "manual",
    };

    SyncConfigFfi {
        base_url: c.base_url.clone(),
        access_token: c.access_token.clone(),
        local_root: c.local_root.to_string_lossy().to_string(),
        remote_root: c.remote_root.clone(),
        sync_mode: sync_mode.to_string(),
        conflict_strategy: conflict_strategy.to_string(),
        max_concurrent_transfers: c.max_concurrent_transfers as u32,
        bandwidth_limit_kbps: c.bandwidth_limit.map(|b| b / 1024).unwrap_or(0),
        excluded_paths: c.excluded_paths.clone(),
        selective_dirs: c.selective_dirs.clone(),
    }
}

fn status_to_ffi(s: crate::models::SyncStatusSnapshot) -> SyncStatusFfi {
    let error_msg = if let crate::models::SyncState::Error { ref message } = s.state {
        Some(message.clone())
    } else {
        s.error_message
    };

    let state = match s.state {
        crate::models::SyncState::Idle => "idle".to_string(),
        crate::models::SyncState::Initializing => "initializing".to_string(),
        crate::models::SyncState::InitialSync { .. } => "initialSync".to_string(),
        crate::models::SyncState::Continuous => "continuous".to_string(),
        crate::models::SyncState::Paused => "paused".to_string(),
        crate::models::SyncState::Error { .. } => "error".to_string(),
        crate::models::SyncState::Stopped => "stopped".to_string(),
    };

    SyncStatusFfi {
        state,
        synced_files: s.synced_files,
        total_files: s.total_files,
        uploading_count: s.uploading_count,
        downloading_count: s.downloading_count,
        conflict_count: s.conflict_count,
        error_count: s.error_count,
        last_sync_time: s.last_sync_time,
        error_message: error_msg,
    }
}

fn summary_to_ffi(s: crate::models::SyncSummary) -> SyncSummaryFfi {
    SyncSummaryFfi {
        uploaded: s.uploaded,
        downloaded: s.downloaded,
        conflicts: s.conflicts,
        skipped: s.skipped,
        deleted_local: s.deleted_local,
        deleted_remote: s.deleted_remote,
        duration_ms: s.duration_ms,
    }
}

fn album_result_to_ffi(r: crate::models::CloudAlbumCheckResult) -> CloudAlbumCheckResultFfi {
    CloudAlbumCheckResultFfi {
        dcim_exists: r.dcim_exists,
        pictures_exists: r.pictures_exists,
        dcim_uri: r.dcim_uri,
        pictures_uri: r.pictures_uri,
    }
}

// ========== 生命周期 ==========

/// 初始化同步引擎
#[frb]
pub async fn init_sync_engine(config: SyncConfigFfi) -> Result<(), SyncErrorFfi> {
    let mut guard = ENGINE.lock().await;
    if guard.is_some() {
        return Err(SyncErrorFfi::InternalError {
            message: "引擎已初始化".to_string(),
        });
    }
    let engine = SyncEngine::new(config_from_ffi(config)).await
        .map_err(error_to_ffi)?;
    *guard = Some(engine);
    Ok(())
}

/// 销毁同步引擎
#[frb]
pub async fn dispose_sync_engine() -> Result<(), SyncErrorFfi> {
    if let Some(engine) = ENGINE.lock().await.take() {
        engine.shutdown().await.map_err(error_to_ffi)?;
    }
    Ok(())
}

// ========== 同步控制 ==========

/// 执行初始全量同步
#[frb]
pub async fn start_initial_sync() -> Result<SyncSummaryFfi, SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.run_initial_sync().await
        .map(summary_to_ffi)
        .map_err(error_to_ffi)
}

/// 启动持续同步
#[frb]
pub async fn start_continuous_sync() -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.run_continuous().await.map_err(error_to_ffi)
}

/// 停止同步
#[frb]
pub async fn stop_sync() -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    if let Some(engine) = guard.as_ref() {
        engine.stop().await.map_err(error_to_ffi)?;
    }
    Ok(())
}

/// 暂停同步
#[frb]
pub async fn pause_sync() -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    if let Some(engine) = guard.as_ref() {
        engine.pause().await.map_err(error_to_ffi)?;
    }
    Ok(())
}

/// 恢复同步
#[frb]
pub async fn resume_sync() -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    if let Some(engine) = guard.as_ref() {
        engine.resume().await.map_err(error_to_ffi)?;
    }
    Ok(())
}

/// 强制同步（重新扫描全量差异）
#[frb]
pub async fn force_sync() -> Result<SyncSummaryFfi, SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.force_sync().await
        .map(summary_to_ffi)
        .map_err(error_to_ffi)
}

// ========== 状态查询 ==========

/// 获取同步状态快照
#[frb]
pub async fn get_sync_status() -> Result<SyncStatusFfi, SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    Ok(status_to_ffi(engine.status()))
}

/// 获取同步配置
#[frb]
pub async fn get_sync_config() -> Result<SyncConfigFfi, SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    Ok(config_to_ffi(&engine.config()))
}

/// 更新同步配置
#[frb]
pub async fn update_sync_config(config: SyncConfigFfi) -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    if let Some(engine) = guard.as_ref() {
        engine.update_config(config_from_ffi(config)).await.map_err(error_to_ffi)?;
    }
    Ok(())
}

// ========== Token 管理 ==========

/// Dart 推送新 Token 给 Rust
#[frb]
pub async fn update_tokens(access_token: String) -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    if let Some(engine) = guard.as_ref() {
        engine.update_access_token(access_token).await;
    }
    Ok(())
}

// ========== Windows 专用 ==========

/// 水合文件（Windows 按需下载）
#[frb]
pub async fn hydrate_file(local_path: String) -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.hydrate_file(&local_path).await.map_err(error_to_ffi)
}

// ========== Android 专用 ==========

/// 同步相册到云端
#[frb]
pub async fn sync_album_to_cloud(
    album_paths: Vec<String>,
    remote_dcim_uri: String,
) -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.sync_album(album_paths, &remote_dcim_uri).await.map_err(error_to_ffi)
}

/// 检查云端是否存在 DCIM/Pictures 目录
#[frb]
pub async fn check_cloud_album_dirs(base_uri: String) -> Result<CloudAlbumCheckResultFfi, SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.check_album_dirs(&base_uri).await
        .map(album_result_to_ffi)
        .map_err(error_to_ffi)
}

/// 在云端创建 DCIM/Pictures 目录
#[frb]
pub async fn create_cloud_album_dirs(base_uri: String) -> Result<(), SyncErrorFfi> {
    let guard = ENGINE.lock().await;
    let engine = guard.as_ref().ok_or(SyncErrorFfi::NotInitialized)?;
    engine.create_album_dirs(&base_uri).await.map_err(error_to_ffi)
}
