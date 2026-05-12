use r2d2::CustomizeConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;
use tokio::sync::Mutex;

use crate::errors::{Result, SyncError};
use crate::models::*;

pub struct SyncDb {
    write_conn: Mutex<Connection>,
    read_pool: r2d2::Pool<SqliteConnectionManager>,
}

#[derive(Debug)]
struct SyncDbConnectionCustomizer;

impl CustomizeConnection<Connection, rusqlite::Error> for SyncDbConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> std::result::Result<(), rusqlite::Error> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA synchronous=NORMAL;",
        )?;
        Ok(())
    }
}

impl SyncDb {
    pub fn read_pool(&self) -> r2d2::Pool<SqliteConnectionManager> {
        self.read_pool.clone()
    }

    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let write_conn = Connection::open(db_path)?;
        write_conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;
             PRAGMA temp_store=MEMORY;",
        )?;

        let manager = SqliteConnectionManager::file(db_path);
        let read_pool = r2d2::Pool::builder()
            .max_size(4)
            .connection_customizer(Box::new(SyncDbConnectionCustomizer))
            .build(manager)?;

        let db = Self {
            write_conn: Mutex::new(write_conn),
            read_pool,
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.write_conn.blocking_lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_root (
                id              TEXT PRIMARY KEY,
                local_path      TEXT NOT NULL UNIQUE,
                remote_uri      TEXT NOT NULL,
                sync_mode       TEXT NOT NULL DEFAULT 'full',
                enabled         INTEGER NOT NULL DEFAULT 1,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_mapping (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_root_id    TEXT NOT NULL REFERENCES sync_root(id),
                local_path      TEXT NOT NULL,
                remote_uri      TEXT NOT NULL,
                remote_file_id  TEXT,
                local_hash      TEXT,
                remote_hash     TEXT,
                local_mtime     INTEGER,
                remote_mtime    INTEGER,
                local_size      INTEGER,
                remote_size     INTEGER,
                sync_status     TEXT NOT NULL DEFAULT 'synced',
                is_placeholder  INTEGER NOT NULL DEFAULT 0,
                updated_at      TEXT NOT NULL,
                UNIQUE(sync_root_id, local_path)
            );

            CREATE INDEX IF NOT EXISTS idx_file_mapping_remote ON file_mapping(sync_root_id, remote_uri);
            CREATE INDEX IF NOT EXISTS idx_file_mapping_status ON file_mapping(sync_status);

            CREATE TABLE IF NOT EXISTS conflict (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_root_id    TEXT NOT NULL REFERENCES sync_root(id),
                local_path      TEXT NOT NULL,
                conflict_type   TEXT NOT NULL,
                resolution      TEXT,
                local_hash      TEXT,
                remote_hash     TEXT,
                created_at      TEXT NOT NULL,
                resolved_at     TEXT
            );

            CREATE TABLE IF NOT EXISTS transfer_queue (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_root_id    TEXT NOT NULL REFERENCES sync_root(id),
                file_mapping_id INTEGER REFERENCES file_mapping(id),
                direction       TEXT NOT NULL,
                local_path      TEXT NOT NULL,
                remote_uri      TEXT NOT NULL,
                file_size       INTEGER NOT NULL,
                bytes_done      INTEGER NOT NULL DEFAULT 0,
                status          TEXT NOT NULL DEFAULT 'pending',
                retry_count     INTEGER NOT NULL DEFAULT 0,
                max_retries     INTEGER NOT NULL DEFAULT 5,
                error_message   TEXT,
                session_id      TEXT,
                chunk_index     INTEGER,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_transfer_status ON transfer_queue(status);

            CREATE TABLE IF NOT EXISTS sync_log (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_root_id    TEXT NOT NULL,
                event_type      TEXT NOT NULL,
                details         TEXT,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS album_sync_record (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                local_path      TEXT NOT NULL UNIQUE,
                remote_uri      TEXT NOT NULL,
                file_hash       TEXT,
                synced_at       TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    // ===== sync_root 操作 =====

    pub async fn upsert_sync_root(&self, root: &SyncConfig) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let local_path = root.local_root.to_string_lossy().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let sync_mode = match root.sync_mode {
            SyncMode::Full => "full",
            SyncMode::Selective => "selective",
            SyncMode::Album => "album",
        };

        let conn = self.write_conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO sync_root (id, local_path, remote_uri, sync_mode, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
            rusqlite::params![id, local_path, root.remote_root, sync_mode, now],
        )?;
        Ok(id)
    }

    // ===== file_mapping 操作 =====

    pub async fn get_file_mapping(&self, sync_root_id: &str, local_path: &str) -> Result<Option<FileMapping>> {
        let pool = self.read_pool.clone();
        let sync_root_id = sync_root_id.to_string();
        let local_path = local_path.to_string();

        let result = tokio::task::spawn_blocking(move || -> Result<Option<FileMapping>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, sync_root_id, local_path, remote_uri, remote_file_id,
                        local_hash, remote_hash, local_mtime, remote_mtime,
                        local_size, remote_size, sync_status, is_placeholder
                 FROM file_mapping WHERE sync_root_id = ?1 AND local_path = ?2"
            )?;

            let mapping = stmt.query_row(
                rusqlite::params![sync_root_id, local_path],
                |row| {
                    Ok(FileMapping {
                        id: row.get(0)?,
                        sync_root_id: row.get(1)?,
                        local_path: std::path::PathBuf::from(row.get::<_, String>(2)?),
                        remote_uri: row.get(3)?,
                        remote_file_id: row.get(4)?,
                        local_hash: row.get(5)?,
                        remote_hash: row.get(6)?,
                        local_mtime: row.get(7)?,
                        remote_mtime: row.get(8)?,
                        local_size: row.get(9)?,
                        remote_size: row.get(10)?,
                        sync_status: parse_sync_status(&row.get::<_, String>(11)?),
                        is_placeholder: row.get::<_, i32>(12)? != 0,
                    })
                },
            ).ok();

            Ok(mapping)
        }).await??;

        Ok(result)
    }

    pub async fn upsert_file_mapping(&self, mapping: &FileMapping) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        let status_str = sync_status_str(&mapping.sync_status);
        let local_path = mapping.local_path.to_string_lossy().to_string();
        let is_placeholder = if mapping.is_placeholder { 1 } else { 0 };

        conn.execute(
            "INSERT OR REPLACE INTO file_mapping
             (sync_root_id, local_path, remote_uri, remote_file_id,
              local_hash, remote_hash, local_mtime, remote_mtime,
              local_size, remote_size, sync_status, is_placeholder, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                mapping.sync_root_id,
                local_path,
                mapping.remote_uri,
                mapping.remote_file_id,
                mapping.local_hash,
                mapping.remote_hash,
                mapping.local_mtime,
                mapping.remote_mtime,
                mapping.local_size,
                mapping.remote_size,
                status_str,
                is_placeholder,
                now,
            ],
        )?;
        Ok(())
    }

    pub async fn delete_file_mapping(&self, sync_root_id: &str, local_path: &str) -> Result<bool> {
        let conn = self.write_conn.lock().await;
        let rows = conn.execute(
            "DELETE FROM file_mapping WHERE sync_root_id = ?1 AND local_path = ?2",
            rusqlite::params![sync_root_id, local_path],
        )?;
        Ok(rows > 0)
    }

    // ===== transfer_queue 操作 =====

    pub async fn add_transfer(&self, task: &TransferTask) -> Result<i64> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        let direction = match task.direction {
            TransferDirection::Upload => "upload",
            TransferDirection::Download => "download",
        };
        let status = transfer_status_str(&task.status);
        let local_path = task.local_path.to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO transfer_queue
             (sync_root_id, file_mapping_id, direction, local_path, remote_uri,
              file_size, bytes_done, status, retry_count, max_retries,
              error_message, session_id, chunk_index, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            rusqlite::params![
                task.sync_root_id,
                task.file_mapping_id,
                direction,
                local_path,
                task.remote_uri,
                task.file_size,
                task.bytes_done,
                status,
                task.retry_count,
                task.max_retries,
                task.error_message,
                task.session_id,
                task.chunk_index,
                now,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub async fn update_transfer_progress(&self, id: i64, bytes_done: u64) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE transfer_queue SET bytes_done = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![bytes_done, now, id],
        )?;
        Ok(())
    }

    pub async fn complete_transfer(&self, id: i64) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE transfer_queue SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    pub async fn fail_transfer(&self, id: i64, error: &str) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE transfer_queue SET status = 'failed', error_message = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![error, now, id],
        )?;
        Ok(())
    }

    pub async fn get_interrupted_transfers(&self) -> Result<Vec<TransferTask>> {
        let pool = self.read_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<TransferTask>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, sync_root_id, file_mapping_id, direction, local_path, remote_uri,
                        file_size, bytes_done, status, retry_count, max_retries,
                        error_message, session_id, chunk_index
                 FROM transfer_queue WHERE status IN ('active', 'pending')"
            )?;
            let tasks = stmt.query_map([], |row| {
                Ok(transfer_task_from_row(row))
            })?.filter_map(|t| t.ok()).collect();
            Ok(tasks)
        }).await??;
        Ok(result)
    }

    // ===== album_sync_record 操作 =====

    pub async fn get_album_sync_records(&self) -> Result<std::collections::HashMap<String, String>> {
        let pool = self.read_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<std::collections::HashMap<String, String>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare("SELECT local_path, remote_uri FROM album_sync_record")?;
            let records = stmt.query_map([], |row| {
                let local: String = row.get(0)?;
                let remote: String = row.get(1)?;
                Ok((local, remote))
            })?.filter_map(|r| r.ok()).collect();
            Ok(records)
        }).await??;
        Ok(result)
    }

    pub async fn add_album_sync_record(&self, local_path: &str, remote_uri: &str, file_hash: &str) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO album_sync_record (local_path, remote_uri, file_hash, synced_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![local_path, remote_uri, file_hash, now],
        )?;
        Ok(())
    }

    // ===== sync_log 操作 =====

    pub async fn add_log(&self, sync_root_id: &str, event_type: &str, details: Option<&str>) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sync_log (sync_root_id, event_type, details, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![sync_root_id, event_type, details, now],
        )?;
        Ok(())
    }
}

fn parse_sync_status(s: &str) -> SyncFileStatus {
    match s {
        "uploading" => SyncFileStatus::Uploading,
        "downloading" => SyncFileStatus::Downloading,
        "conflict" => SyncFileStatus::Conflict,
        "placeholder" => SyncFileStatus::Placeholder,
        _ => SyncFileStatus::Synced,
    }
}

fn sync_status_str(s: &SyncFileStatus) -> &'static str {
    match s {
        SyncFileStatus::Synced => "synced",
        SyncFileStatus::Uploading => "uploading",
        SyncFileStatus::Downloading => "downloading",
        SyncFileStatus::Conflict => "conflict",
        SyncFileStatus::Placeholder => "placeholder",
    }
}

fn transfer_status_str(s: &TransferStatus) -> &'static str {
    match s {
        TransferStatus::Pending => "pending",
        TransferStatus::Active => "active",
        TransferStatus::Paused => "paused",
        TransferStatus::Completed => "completed",
        TransferStatus::Failed => "failed",
    }
}

fn transfer_task_from_row(row: &rusqlite::Row<'_>) -> TransferTask {
    let direction_str: String = row.get(3).unwrap_or_default();
    let direction = if direction_str == "upload" {
        TransferDirection::Upload
    } else {
        TransferDirection::Download
    };
    let status_str: String = row.get(8).unwrap_or_default();
    let status = match status_str.as_str() {
        "active" => TransferStatus::Active,
        "paused" => TransferStatus::Paused,
        "completed" => TransferStatus::Completed,
        "failed" => TransferStatus::Failed,
        _ => TransferStatus::Pending,
    };

    TransferTask {
        id: row.get(0).unwrap_or(0),
        sync_root_id: row.get(1).unwrap_or_default(),
        file_mapping_id: row.get(2).ok(),
        direction,
        local_path: std::path::PathBuf::from(row.get::<_, String>(4).unwrap_or_default()),
        remote_uri: row.get(5).unwrap_or_default(),
        file_size: row.get(6).unwrap_or(0),
        bytes_done: row.get(7).unwrap_or(0),
        status,
        retry_count: row.get(9).unwrap_or(0),
        max_retries: row.get(10).unwrap_or(5),
        error_message: row.get(11).ok(),
        session_id: row.get(12).ok(),
        chunk_index: row.get(13).ok(),
    }
}
