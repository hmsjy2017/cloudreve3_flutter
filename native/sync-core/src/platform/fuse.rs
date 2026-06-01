//! Linux FUSE 平台适配器
//! 仅在 linux-fuse feature 启用时编译
//!
//! 提供云端文件系统的 FUSE 镜像挂载，支持：
//! - 按需水合（read 时下载）
//! - 远程事件 → inode 增量更新
//! - 懒加载目录列表（readdir 时从远程 API 拉取）

#![cfg(feature = "linux-fuse")]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyStatfs,
    Request,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::api_client::ApiClient;
use crate::sync_db::SyncDb;
use crate::models::SyncConfig;

// ========== Inode 存储 ==========

/// FUSE inode 号常量
const FUSE_ROOT_INO: u64 = 1;

/// 内存中的 inode 条目
#[derive(Debug, Clone)]
pub struct InodeEntry {
    pub ino: u64,
    pub parent_ino: u64,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_ms: i64,
    pub remote_uri: String,
    pub remote_hash: Option<String>,
    /// 目录是否已从远程加载过子项
    pub dir_loaded: bool,
}

/// 内存中的 inode 表
pub struct InodeStore {
    /// inode → 文件元数据
    entries: DashMap<u64, InodeEntry>,
    /// 相对路径 → inode（反向索引）
    path_to_inode: DashMap<String, u64>,
    /// 父 inode → 子 inode 名称列表
    children: DashMap<u64, Vec<(String, u64)>>,
    /// 下一个可用 inode 号
    next_inode: AtomicU64,
}

impl Default for InodeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InodeStore {
    pub fn new() -> Self {
        let store = Self {
            entries: DashMap::new(),
            path_to_inode: DashMap::new(),
            children: DashMap::new(),
            next_inode: AtomicU64::new(FUSE_ROOT_INO + 1),
        };
        // 插入根目录
        store.entries.insert(FUSE_ROOT_INO, InodeEntry {
            ino: FUSE_ROOT_INO,
            parent_ino: FUSE_ROOT_INO,
            name: String::new(),
            is_dir: true,
            size: 0,
            mtime_ms: 0,
            remote_uri: String::new(),
            remote_hash: None,
            dir_loaded: false,
        });
        store.path_to_inode.insert(String::new(), FUSE_ROOT_INO);
        store
    }

    /// 分配新的 inode 号
    fn alloc_inode(&self) -> u64 {
        self.next_inode.fetch_add(1, Ordering::Relaxed)
    }

    /// 插入或更新 inode
    pub fn upsert(&self, relative_path: &str, parent_ino: u64, name: &str, entry: InodeEntry) -> u64 {
        if let Some(existing_ino) = self.path_to_inode.get(relative_path).map(|r| *r.value()) {
            // 更新已有 inode
            if let Some(mut e) = self.entries.get_mut(&existing_ino) {
                e.size = entry.size;
                e.mtime_ms = entry.mtime_ms;
                e.remote_uri = entry.remote_uri;
                e.remote_hash = entry.remote_hash;
            }
            existing_ino
        } else {
            let ino = entry.ino;
            self.entries.insert(ino, entry);
            self.path_to_inode.insert(relative_path.to_string(), ino);
            // 添加到父目录的子项列表
            self.children.entry(parent_ino).or_default().push((name.to_string(), ino));
            ino
        }
    }

    /// 移除 inode
    pub fn remove(&self, relative_path: &str) -> Option<InodeEntry> {
        let (_, ino) = self.path_to_inode.remove(relative_path)?;
        let (_, entry) = self.entries.remove(&ino)?;
        // 从父目录子项中移除
        if let Some(mut children) = self.children.get_mut(&entry.parent_ino) {
            children.retain(|(child_name, child_ino)| !(*child_ino == ino && child_name == &entry.name));
        }
        // 递归移除子项（如果是目录）
        if entry.is_dir {
            self.remove_children_recursive(ino);
        }
        Some(entry)
    }

    /// 递归移除子项
    fn remove_children_recursive(&self, parent_ino: u64) {
        let (_, child_list) = match self.children.remove(&parent_ino) {
            Some(pair) => pair,
            None => return,
        };
        for (child_name, child_ino) in &child_list {
            let _ = child_name; // used in iteration
            if let Some((_, child_entry)) = self.entries.remove(child_ino) {
                // 从 path_to_inode 中移除（通过反向查找）
                let mut to_remove = None;
                for item in self.path_to_inode.iter() {
                    if *item.value() == *child_ino {
                        to_remove = Some(item.key().clone());
                        break;
                    }
                }
                if let Some(path) = to_remove {
                    self.path_to_inode.remove(&path);
                }
                if child_entry.is_dir {
                    self.remove_children_recursive(*child_ino);
                }
            }
        }
    }

    /// 获取 inode 条目
    pub fn get(&self, ino: u64) -> Option<InodeEntry> {
        self.entries.get(&ino).map(|r| r.value().clone())
    }

    /// 获取子项列表
    pub fn get_children(&self, parent_ino: u64) -> Vec<(String, u64)> {
        self.children.get(&parent_ino).map(|r| r.value().clone()).unwrap_or_default()
    }

    /// 根据父 inode 和名称查找子项
    pub fn lookup_child(&self, parent_ino: u64, name: &str) -> Option<InodeEntry> {
        let children = self.get_children(parent_ino);
        for (child_name, child_ino) in &children {
            if child_name == name {
                return self.get(*child_ino);
            }
        }
        None
    }
}

// ========== FUSE 请求/响应 ==========

/// FUSE 水合请求（FUSE read handler → SyncEngine）
pub struct FuseFetchRequest {
    pub inode: u64,
    pub remote_uri: String,
    pub offset: i64,
    pub length: i64,
    pub reply_tx: tokio::sync::oneshot::Sender<Result<Vec<u8>, String>>,
}

// ========== FUSE 文件系统实现 ==========

/// FUSE 文件系统
struct CloudreveFuseFs {
    inode_store: Arc<InodeStore>,
    fetch_tx: mpsc::Sender<FuseFetchRequest>,
    runtime: tokio::runtime::Handle,
}

impl CloudreveFuseFs {
    fn new(
        inode_store: Arc<InodeStore>,
        fetch_tx: mpsc::Sender<FuseFetchRequest>,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self { inode_store, fetch_tx, runtime }
    }

    fn entry_to_attr(&self, entry: &InodeEntry) -> FileAttr {
        let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_millis(entry.mtime_ms as u64);
        let kind = if entry.is_dir { FileType::Directory } else { FileType::RegularFile };
        FileAttr {
            ino: entry.ino,
            size: entry.size,
            blocks: entry.size.div_ceil(512),
            atime: mtime,
            mtime,
            ctime: mtime,
            crtime: mtime,
            kind,
            perm: if entry.is_dir { 0o755 } else { 0o644 },
            nlink: if entry.is_dir { 2 } else { 1 },
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            flags: 0,
            blksize: 4096,
        }
    }
}

impl Filesystem for CloudreveFuseFs {
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        match self.inode_store.get(ino) {
            Some(entry) => reply.attr(&std::time::Duration::from_secs(1), &self.entry_to_attr(&entry)),
            None => reply.error(libc::ENOENT),
        }
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &std::ffi::OsStr, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        match self.inode_store.lookup_child(parent, &name_str) {
            Some(entry) => reply.entry(&std::time::Duration::from_secs(1), &self.entry_to_attr(&entry), 0),
            None => reply.error(libc::ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entry = match self.inode_store.get(ino) {
            Some(e) => e,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        if !entry.is_dir {
            reply.error(libc::ENOTDIR);
            return;
        }

        // . 和 .. 条目
        if offset == 0
            && reply.add(entry.ino, 1, FileType::Directory, ".")
        {
            reply.ok();
            return;
        }
        if offset <= 1 {
            let parent_ino = if entry.ino == FUSE_ROOT_INO { FUSE_ROOT_INO } else { entry.parent_ino };
            if reply.add(parent_ino, 2, FileType::Directory, "..") {
                reply.ok();
                return;
            }
        }

        // 子项
        let children = self.inode_store.get_children(ino);
        for (i, (name, child_ino)) in children.iter().enumerate() {
            let idx = (i + 2) as i64; // 偏移 0=., 1=.., 2+=子项
            if idx < offset {
                continue;
            }
            let child_entry = match self.inode_store.get(*child_ino) {
                Some(e) => e,
                None => continue,
            };
            let kind = if child_entry.is_dir { FileType::Directory } else { FileType::RegularFile };
            // +1 是下一个 offset
            if reply.add(*child_ino, idx + 1, kind, name.as_str()) {
                break;
            }
        }
        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let entry = match self.inode_store.get(ino) {
            Some(e) => e,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        if entry.is_dir {
            reply.error(libc::EISDIR);
            return;
        }

        if entry.remote_uri.is_empty() {
            reply.error(libc::EIO);
            return;
        }

        // 发送水合请求到 SyncEngine
        let (tx, rx) = tokio::sync::oneshot::channel();
        let request = FuseFetchRequest {
            inode: ino,
            remote_uri: entry.remote_uri.clone(),
            offset,
            length: size as i64,
            reply_tx: tx,
        };

        if self.fetch_tx.blocking_send(request).is_err() {
            tracing::error!("FUSE read: 水合请求发送失败（通道已关闭）");
            reply.error(libc::EIO);
            return;
        }

        // 阻塞等待 SyncEngine 下载完成
        match self.runtime.block_on(rx) {
            Ok(Ok(data)) => {
                let start = offset as usize;
                let end = (offset as usize + size as usize).min(data.len());
                if start < data.len() {
                    reply.data(&data[start..end]);
                } else {
                    reply.data(&[]);
                }
            }
            Ok(Err(e)) => {
                tracing::error!("FUSE read: 水合失败: {}", e);
                reply.error(libc::EIO);
            }
            Err(_) => {
                tracing::error!("FUSE read: 水合请求被丢弃");
                reply.error(libc::EIO);
            }
        }
    }

    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(
            0,       // blocks
            0,       // bfree
            0,       // bavail
            0,       // files
            0,       // ffree
            4096,    // bsize
            255,     // namelen
            0,       // frsize
        );
    }

    fn init(
        &mut self,
        _req: &Request,
        _config: &mut fuser::KernelConfig,
    ) -> std::result::Result<(), libc::c_int> {
        tracing::info!("FUSE 文件系统已初始化");
        Ok(())
    }

    fn destroy(&mut self) {
        tracing::info!("FUSE 文件系统已销毁");
    }
}

// ========== FUSE 平台适配器 ==========

pub struct FusePlatformAdapter {
    mount_path: std::path::PathBuf,
    inode_store: Arc<InodeStore>,
    fetch_rx: std::sync::Mutex<Option<mpsc::Receiver<FuseFetchRequest>>>,
    shutdown: CancellationToken,
    #[allow(dead_code)]
    runtime: tokio::runtime::Handle,
    #[allow(dead_code)]
    db: Arc<SyncDb>,
    #[allow(dead_code)]
    api: Arc<ApiClient>,
    #[allow(dead_code)]
    config: SyncConfig,
}

impl FusePlatformAdapter {
    pub fn new(
        mount_path: &Path,
        db: Arc<SyncDb>,
        api: Arc<ApiClient>,
        config: SyncConfig,
    ) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Handle::current();
        let inode_store = Arc::new(InodeStore::new());
        let (fetch_tx, fetch_rx) = mpsc::channel::<FuseFetchRequest>(64);
        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        let mount_path_buf = mount_path.to_path_buf();

        // 创建挂载目录
        std::fs::create_dir_all(mount_path)
            .map_err(|e| anyhow::anyhow!("创建 FUSE 挂载目录失败: {}", e))?;

        let inode_store_clone = inode_store.clone();
        let mount_path_clone = mount_path_buf.clone();
        let runtime_for_thread = runtime.clone();

        // 启动 FUSE 挂载线程
        std::thread::Builder::new()
            .name("fuse-mount".to_string())
            .spawn(move || {
                let fs = CloudreveFuseFs::new(
                    inode_store_clone,
                    fetch_tx,
                    runtime_for_thread,
                );

                let options = vec![
                    fuser::MountOption::FSName("cloudreve".to_string()),
                    fuser::MountOption::Subtype("cloudreve".to_string()),
                    fuser::MountOption::RO,       // 只读挂载
                    fuser::MountOption::NoAtime,
                ];

                tracing::info!("FUSE 挂载中: {}", mount_path_clone.display());

                match fuser::mount2(fs, &mount_path_clone, &options) {
                    Ok(()) => tracing::info!("FUSE 挂载已结束: {}", mount_path_clone.display()),
                    Err(e) => {
                        if shutdown_clone.is_cancelled() {
                            tracing::info!("FUSE 挂载已关闭（正常退出）");
                        } else {
                            tracing::error!("FUSE 挂载失败: {}", e);
                        }
                    }
                }
            })
            .map_err(|e| anyhow::anyhow!("启动 FUSE 线程失败: {}", e))?;

        tracing::info!("FusePlatformAdapter 初始化完成: {}", mount_path.display());

        Ok(Self {
            mount_path: mount_path_buf,
            inode_store,
            fetch_rx: std::sync::Mutex::new(Some(fetch_rx)),
            shutdown,
            runtime,
            db,
            api,
            config,
        })
    }

    /// 取走水合请求接收端（供 SyncEngine 持续同步消费）
    pub fn take_fetch_receiver(&self) -> Option<mpsc::Receiver<FuseFetchRequest>> {
        self.fetch_rx.lock().ok().and_then(|mut rx| rx.take())
    }

    /// 获取 inode 存储（供远程事件更新）
    pub fn inode_store(&self) -> &Arc<InodeStore> {
        &self.inode_store
    }

    /// 在 FUSE inode 缓存中注册远程文件
    #[allow(clippy::too_many_arguments)]
    pub fn create_placeholder_for_remote(
        &self,
        parent_rel: &str,
        name: &str,
        relative_path: &str,
        is_dir: bool,
        size: u64,
        remote_uri: &str,
        remote_hash: Option<&str>,
        mtime_ms: i64,
    ) {
        let parent_ino = self.inode_store.path_to_inode.get(parent_rel)
            .map(|r| *r.value())
            .unwrap_or(FUSE_ROOT_INO);

        let ino = self.inode_store.alloc_inode();
        let entry = InodeEntry {
            ino,
            parent_ino,
            name: name.to_string(),
            is_dir,
            size,
            mtime_ms,
            remote_uri: remote_uri.to_string(),
            remote_hash: remote_hash.map(|s| s.to_string()),
            dir_loaded: false,
        };

        self.inode_store.upsert(relative_path, parent_ino, name, entry);
        tracing::debug!("FUSE inode 注册: {} (ino={}, dir={})", relative_path, ino, is_dir);
    }

    /// 从 FUSE inode 缓存中移除文件
    pub fn remove_inode(&self, relative_path: &str) {
        if let Some(entry) = self.inode_store.remove(relative_path) {
            tracing::debug!("FUSE inode 移除: {} (ino={})", relative_path, entry.ino);
        }
    }

    /// 卸载 FUSE 文件系统（lazy unmount，允许 busy 时延迟卸载）
    pub fn unmount(&self) -> anyhow::Result<()> {
        self.shutdown.cancel();

        // 优先尝试 fusermount3 -uz（lazy unmount），回退到 fusermount -uz
        let result = std::process::Command::new("fusermount3")
            .args(["-u", "-z"])
            .arg(&self.mount_path)
            .status()
            .or_else(|_| {
                std::process::Command::new("fusermount")
                    .args(["-u", "-z"])
                    .arg(&self.mount_path)
                    .status()
            });

        match result {
            Ok(s) if s.success() => {
                tracing::info!("FUSE 已卸载: {}", self.mount_path.display());
                Ok(())
            }
            Ok(s) => {
                tracing::warn!("FUSE 卸载退出码: {}", s);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("FUSE 卸载命令失败: {}", e);
                Ok(())
            }
        }
    }

    /// 获取挂载路径
    pub fn mount_path(&self) -> &Path {
        &self.mount_path
    }
}
