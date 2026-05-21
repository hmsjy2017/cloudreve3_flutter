use r2d2::CustomizeConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;
use tokio::sync::Mutex;

use crate::errors::Result;
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

        Self::run_migrations(&write_conn)?;

        let manager = SqliteConnectionManager::file(db_path);
        let read_pool = r2d2::Pool::builder()
            .max_size(4)
            .connection_customizer(Box::new(SyncDbConnectionCustomizer))
            .build(manager)?;

        Ok(Self {
            write_conn: Mutex::new(write_conn),
            read_pool,
        })
    }

    fn run_migrations(conn: &Connection) -> Result<()> {
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
            );

            CREATE TABLE IF NOT EXISTS sync_task (
                id              TEXT PRIMARY KEY,
                trigger         TEXT NOT NULL,
                total_count     INTEGER NOT NULL DEFAULT 0,
                completed_count INTEGER NOT NULL DEFAULT 0,
                failed_count    INTEGER NOT NULL DEFAULT 0,
                status          TEXT NOT NULL DEFAULT 'pending',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL,
                finished_at     TEXT
            );

            CREATE TABLE IF NOT EXISTS sync_task_item (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id         TEXT NOT NULL REFERENCES sync_task(id),
                relative_path   TEXT NOT NULL,
                action_type     TEXT NOT NULL,
                status          TEXT NOT NULL DEFAULT 'pending',
                file_size       INTEGER NOT NULL DEFAULT 0,
                error_message   TEXT,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_task_item_task ON sync_task_item(task_id);
            CREATE INDEX IF NOT EXISTS idx_task_item_path ON sync_task_item(relative_path);
            CREATE INDEX IF NOT EXISTS idx_task_item_action ON sync_task_item(action_type);
            CREATE INDEX IF NOT EXISTS idx_task_item_status ON sync_task_item(status);
            CREATE INDEX IF NOT EXISTS idx_task_status ON sync_task(status);",
        )?;
        Ok(())
    }

    // ===== sync_root 操作 =====

    pub async fn upsert_sync_root(&self, root: &SyncConfig) -> Result<String> {
        let local_path = root.local_root.to_string_lossy().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let sync_mode = match root.sync_mode {
            SyncMode::Full => "full",
            SyncMode::UploadOnly => "upload_only",
            SyncMode::DownloadOnly => "download_only",
            SyncMode::Album => "album",
        };

        let conn = self.write_conn.lock().await;

        // 先查找是否已存在
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM sync_root WHERE local_path = ?1",
                rusqlite::params![local_path],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            // 更新已有记录，保留 id 和 created_at
            conn.execute(
                "UPDATE sync_root SET remote_uri = ?1, sync_mode = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![root.remote_root, sync_mode, now, id],
            )?;
            Ok(id)
        } else {
            // 新建记录
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO sync_root (id, local_path, remote_uri, sync_mode, enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                rusqlite::params![id, local_path, root.remote_root, sync_mode, now],
            )?;
            Ok(id)
        }
    }

    // ===== file_mapping 操作 =====

    pub async fn get_file_mapping(
        &self,
        sync_root_id: &str,
        local_path: &str,
    ) -> Result<Option<FileMapping>> {
        let pool = self.read_pool.clone();
        let sync_root_id = sync_root_id.to_string();
        let local_path = local_path.to_string();

        let result = tokio::task::spawn_blocking(move || -> Result<Option<FileMapping>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, sync_root_id, local_path, remote_uri, remote_file_id,
                        local_hash, remote_hash, local_mtime, remote_mtime,
                        local_size, remote_size, sync_status, is_placeholder
                 FROM file_mapping WHERE sync_root_id = ?1 AND local_path = ?2",
            )?;

            let mapping = stmt
                .query_row(rusqlite::params![sync_root_id, local_path], |row| {
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
                })
                .ok();

            Ok(mapping)
        })
        .await??;

        Ok(result)
    }

    pub async fn upsert_file_mapping(&self, mapping: &FileMapping) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        let status_str = sync_status_str(&mapping.sync_status);
        let local_path = crate::utils::normalize_path(&mapping.local_path.to_string_lossy());
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

    /// 更新文件映射的路径（重命名时使用）
    pub async fn update_file_mapping_path(
        &self,
        sync_root_id: &str,
        old_relative_path: &str,
        new_relative_path: &str,
        new_remote_uri: &str,
    ) -> Result<bool> {
        let conn = self.write_conn.lock().await;
        let rows = conn.execute(
            "UPDATE file_mapping SET local_path = ?1, remote_uri = ?2 WHERE sync_root_id = ?3 AND local_path = ?4",
            rusqlite::params![new_relative_path, new_remote_uri, sync_root_id, old_relative_path],
        )?;
        Ok(rows > 0)
    }

    /// 删除指定目录前缀下的所有文件映射（含目录自身）
    pub async fn delete_file_mapping_prefix(
        &self,
        sync_root_id: &str,
        dir_prefix: &str,
    ) -> Result<u64> {
        let conn = self.write_conn.lock().await;
        // 匹配: dir_prefix 自身、dir_prefix/... 子路径
        let exact = dir_prefix;
        let child_prefix = format!("{}/", dir_prefix);
        let rows = conn.execute(
            "DELETE FROM file_mapping WHERE sync_root_id = ?1 AND (local_path = ?2 OR local_path LIKE ?3 || '%')",
            rusqlite::params![sync_root_id, exact, child_prefix],
        )?;
        Ok(rows as u64)
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
                 FROM transfer_queue WHERE status IN ('active', 'pending')",
            )?;
            let tasks = stmt
                .query_map([], |row| Ok(transfer_task_from_row(row)))?
                .filter_map(|t| t.ok())
                .collect();
            Ok(tasks)
        })
        .await??;
        Ok(result)
    }

    // ===== album_sync_record 操作 =====

    pub async fn get_album_sync_records(
        &self,
    ) -> Result<std::collections::HashMap<String, String>> {
        let pool = self.read_pool.clone();
        let result = tokio::task::spawn_blocking(
            move || -> Result<std::collections::HashMap<String, String>> {
                let conn = pool.get()?;
                let mut stmt =
                    conn.prepare("SELECT local_path, remote_uri FROM album_sync_record")?;
                let records = stmt
                    .query_map([], |row| {
                        let local: String = row.get(0)?;
                        let remote: String = row.get(1)?;
                        Ok((local, remote))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(records)
            },
        )
        .await??;
        Ok(result)
    }

    pub async fn add_album_sync_record(
        &self,
        local_path: &str,
        remote_uri: &str,
        file_hash: &str,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO album_sync_record (local_path, remote_uri, file_hash, synced_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![local_path, remote_uri, file_hash, now],
        )?;
        Ok(())
    }

    // ===== sync_log 操作 =====

    pub async fn add_log(
        &self,
        sync_root_id: &str,
        event_type: &str,
        details: Option<&str>,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sync_log (sync_root_id, event_type, details, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![sync_root_id, event_type, details, now],
        )?;
        Ok(())
    }

    // ===== sync_task 操作 =====

    pub async fn create_sync_task(&self, task: &SyncTask) -> Result<()> {
        let conn = self.write_conn.lock().await;
        conn.execute(
            "INSERT INTO sync_task (id, trigger, total_count, completed_count, failed_count, status, created_at, updated_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                task.id,
                task.trigger.as_str(),
                task.total_count,
                task.completed_count,
                task.failed_count,
                task.status.as_str(),
                task.created_at,
                task.updated_at,
                task.finished_at,
            ],
        )?;
        Ok(())
    }

    pub async fn update_sync_task_status(
        &self,
        task_id: &str,
        status: &WorkerStatus,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task SET status = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![status.as_str(), now, task_id],
        )?;
        Ok(())
    }

    pub async fn update_sync_task_total_count(
        &self,
        task_id: &str,
        total_count: u32,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task SET total_count = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![total_count, now, task_id],
        )?;
        Ok(())
    }

    pub async fn finish_sync_task(
        &self,
        task_id: &str,
        status: &WorkerStatus,
        completed: u32,
        failed: u32,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task SET status = ?1, completed_count = ?2, failed_count = ?3, updated_at = ?4, finished_at = ?5 WHERE id = ?6",
            rusqlite::params![status.as_str(), completed, failed, now, now, task_id],
        )?;
        Ok(())
    }

    pub async fn increment_task_completed(&self, task_id: &str) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task SET completed_count = completed_count + 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        )?;
        Ok(())
    }

    pub async fn increment_task_failed(&self, task_id: &str) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task SET failed_count = failed_count + 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        )?;
        Ok(())
    }

    pub async fn get_active_sync_tasks(&self) -> Result<Vec<SyncTask>> {
        let pool = self.read_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<SyncTask>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, trigger, total_count, completed_count, failed_count, status, created_at, updated_at, finished_at
                 FROM sync_task WHERE status IN ('pending', 'running')
                 ORDER BY created_at DESC"
            )?;
            let tasks = stmt.query_map([], sync_task_from_row)?
                .filter_map(|t| t.ok()).collect();
            Ok(tasks)
        }).await??;
        Ok(result)
    }

    pub async fn get_recent_sync_tasks(&self, limit: u32) -> Result<Vec<SyncTask>> {
        let pool = self.read_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<SyncTask>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, trigger, total_count, completed_count, failed_count, status, created_at, updated_at, finished_at
                 FROM sync_task ORDER BY created_at DESC LIMIT ?1"
            )?;
            let tasks = stmt.query_map(rusqlite::params![limit], sync_task_from_row)?
                .filter_map(|t| t.ok()).collect();
            Ok(tasks)
        }).await??;
        Ok(result)
    }

    // ===== sync_task_item 操作 =====

    pub async fn create_sync_task_item(&self, item: &SyncTaskItem) -> Result<i64> {
        let conn = self.write_conn.lock().await;
        conn.execute(
            "INSERT INTO sync_task_item (task_id, relative_path, action_type, status, file_size, error_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                item.task_id,
                item.relative_path,
                item.action_type.as_str(),
                item.status.as_str(),
                item.file_size,
                item.error_message,
                item.created_at,
                item.updated_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub async fn update_sync_task_item_status(
        &self,
        item_id: i64,
        status: &TaskItemStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task_item SET status = ?1, error_message = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![status.as_str(), error_message, now, item_id],
        )?;
        Ok(())
    }

    /// 按 task_id + relative_path + action_type 更新 task_item 状态
    pub async fn update_task_item_status_by_path(
        &self,
        task_id: &str,
        relative_path: &str,
        action_type: &str,
        status: &TaskItemStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.write_conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sync_task_item SET status = ?1, error_message = ?2, updated_at = ?3 WHERE task_id = ?4 AND relative_path = ?5 AND action_type = ?6",
            rusqlite::params![status.as_str(), error_message, now, task_id, relative_path, action_type],
        )?;
        Ok(())
    }

    pub async fn get_sync_task_items(&self, task_id: &str) -> Result<Vec<SyncTaskItem>> {
        let pool = self.read_pool.clone();
        let task_id = task_id.to_string();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<SyncTaskItem>> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT id, task_id, relative_path, action_type, status, file_size, error_message, created_at, updated_at
                 FROM sync_task_item WHERE task_id = ?1 ORDER BY id"
            )?;
            let items = stmt.query_map(rusqlite::params![task_id], sync_task_item_from_row)?
                .filter_map(|i| i.ok()).collect();
            Ok(items)
        }).await??;
        Ok(result)
    }

    pub async fn query_task_items(&self, filter: &TaskItemFilter) -> Result<Vec<SyncTaskItem>> {
        let pool = self.read_pool.clone();
        let filter = filter.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<SyncTaskItem>> {
            let conn = pool.get()?;
            let mut sql = String::from(
                "SELECT id, task_id, relative_path, action_type, status, file_size, error_message, created_at, updated_at
                 FROM sync_task_item WHERE 1=1"
            );
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(ref task_id) = filter.task_id {
                sql.push_str(" AND task_id = ?");
                params.push(Box::new(task_id.clone()));
            }
            if let Some(ref path_contains) = filter.relative_path_contains {
                sql.push_str(" AND relative_path LIKE ?");
                params.push(Box::new(format!("%{}%", path_contains)));
            }
            if let Some(ref action_type) = filter.action_type {
                sql.push_str(" AND action_type = ?");
                params.push(Box::new(action_type.clone()));
            }
            if let Some(ref status) = filter.status {
                sql.push_str(" AND status = ?");
                params.push(Box::new(status.clone()));
            }

            sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
            params.push(Box::new(filter.limit));
            params.push(Box::new(filter.offset));

            let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;
            let items = stmt.query_map(param_refs.as_slice(), sync_task_item_from_row)?
                .filter_map(|i| i.ok()).collect();
            Ok(items)
        }).await??;
        Ok(result)
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

fn sync_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncTask> {
    let trigger_str: String = row.get(1)?;
    let trigger = match trigger_str.as_str() {
        "initial_sync" => WorkerTrigger::InitialSync,
        "continuous" => WorkerTrigger::Continuous,
        "manual" => WorkerTrigger::Manual,
        _ => WorkerTrigger::Manual,
    };
    let status_str: String = row.get(5)?;
    Ok(SyncTask {
        id: row.get(0)?,
        trigger,
        total_count: row.get(2)?,
        completed_count: row.get(3)?,
        failed_count: row.get(4)?,
        status: status_str
            .parse::<WorkerStatus>()
            .unwrap_or(WorkerStatus::Pending),
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        finished_at: row.get(8)?,
    })
}

fn sync_task_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncTaskItem> {
    let action_str: String = row.get(3)?;
    let status_str: String = row.get(4)?;
    Ok(SyncTaskItem {
        id: row.get(0)?,
        task_id: row.get(1)?,
        relative_path: row.get(2)?,
        action_type: action_str
            .parse::<TaskActionType>()
            .unwrap_or(TaskActionType::Upload),
        status: status_str
            .parse::<TaskItemStatus>()
            .unwrap_or(TaskItemStatus::Pending),
        file_size: row.get(5)?,
        error_message: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
