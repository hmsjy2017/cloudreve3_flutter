/// Windows 平台适配层
///
/// 提供 Cloud Filter API (CFApi) 集成，用于：
/// - 同步根注册/连接
/// - 占位符创建/脱水/水合
/// - 资源管理器集成
///
/// sync-core 通过 feature gate "windows-cfapi" 引入本 crate

pub struct WindowsAdapter;

impl WindowsAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 注册同步根目录 (Phase 4)
    pub fn register_sync_root(&self, root_path: &std::path::Path) -> anyhow::Result<()> {
        // Phase 4: CFApi 注册
        Ok(())
    }

    /// 连接同步根 (Phase 4)
    pub fn connect_sync_root(&mut self, root_path: &std::path::Path) -> anyhow::Result<()> {
        // Phase 4: CFApi 连接 + 回调注册
        Ok(())
    }

    /// 创建占位符 (Phase 4)
    pub fn create_placeholders(&self, entries: &[PlaceholderEntry]) -> anyhow::Result<()> {
        // Phase 4: CfCreatePlaceholders
        Ok(())
    }

    /// 断开连接
    pub fn disconnect(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// 占位符创建信息
pub struct PlaceholderEntry {
    pub relative_path: String,
    pub file_size: u64,
    pub is_dir: bool,
    pub file_identity: Vec<u8>,
}
