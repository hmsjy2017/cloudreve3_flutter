/// Windows 平台适配层 - Cloud Filter API (CFApi) 集成
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use tokio::sync::mpsc;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::CloudFilters::*;
use windows::Win32::Storage::FileSystem::*;

pub struct WindowsAdapter {
    connection_key: Option<CF_CONNECTION_KEY>,
    callback_rx: Option<mpsc::Receiver<PlatformCallbackEvent>>,
}

/// 占位符创建信息
#[derive(Debug, Clone)]
pub struct PlaceholderEntry {
    pub relative_path: String,
    pub file_size: u64,
    pub is_dir: bool,
    pub file_identity: Vec<u8>,
}

/// 平台回调事件
#[derive(Debug, Clone)]
pub enum PlatformCallbackEvent {
    HydrateRequested {
        local_path: String,
        file_identity: Vec<u8>,
    },
}

impl Default for WindowsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAdapter {
    pub fn new() -> Self {
        Self {
            connection_key: None,
            callback_rx: None,
        }
    }

    /// 注册同步根目录
    pub fn register_sync_root(
        &self,
        root_path: &Path,
        provider_name: &str,
        provider_version: &str,
    ) -> anyhow::Result<()> {
        unsafe {
            let sync_root_path = to_pcwstr(root_path);
            let provider_name_w = to_pcwstr_str(provider_name);
            let provider_version_w = to_pcwstr_str(provider_version);

            let provider_name_pcw = PCWSTR(provider_name_w.as_ptr());
            let provider_version_pcw = PCWSTR(provider_version_w.as_ptr());

            let provider_id = GUID::from_values(
                0x4E_5F_8C_D7, 0xA2_B1, 0x43_9F, [0x8C, 0x12, 0xD5, 0xE7, 0xF3, 0x21, 0x9A, 0xB4],
            );

            let root_identity = b"cloudreve-sync-root".to_vec();

            let registration = CF_SYNC_REGISTRATION {
                StructSize: std::mem::size_of::<CF_SYNC_REGISTRATION>() as u32,
                ProviderName: provider_name_pcw,
                ProviderVersion: provider_version_pcw,
                SyncRootIdentity: root_identity.as_ptr() as *const std::os::raw::c_void,
                SyncRootIdentityLength: root_identity.len() as u32,
                FileIdentity: std::ptr::null(),
                FileIdentityLength: 0,
                ProviderId: provider_id,
            };

            let policies = CF_SYNC_POLICIES {
                StructSize: std::mem::size_of::<CF_SYNC_POLICIES>() as u32,
                Hydration: CF_HYDRATION_POLICY {
                    Primary: CF_HYDRATION_POLICY_PARTIAL,
                    Modifier: CF_HYDRATION_POLICY_MODIFIER_NONE,
                },
                Population: CF_POPULATION_POLICY {
                    Primary: CF_POPULATION_POLICY_ALWAYS_FULL,
                    Modifier: CF_POPULATION_POLICY_MODIFIER_NONE,
                },
                InSync: CF_INSYNC_POLICY_NONE,
                HardLink: CF_HARDLINK_POLICY_NONE,
                PlaceholderManagement: CF_PLACEHOLDER_MANAGEMENT_POLICY_DEFAULT,
            };

            CfRegisterSyncRoot(
                PCWSTR(sync_root_path.as_ptr()),
                &registration,
                &policies,
                CF_REGISTER_FLAG_NONE,
            )?;

            tracing::info!("同步根已注册: {}", root_path.display());
            Ok(())
        }
    }

    /// 连接同步根，注册回调
    pub fn connect_sync_root(&mut self, root_path: &Path) -> anyhow::Result<()> {
        let (_tx, rx) = mpsc::channel(256);

        unsafe {
            let sync_root_path = to_pcwstr(root_path);

            let callbacks = [
                CF_CALLBACK_REGISTRATION {
                    Type: CF_CALLBACK_TYPE_FETCH_DATA,
                    Callback: Some(cf_fetch_data_callback),
                },
                CF_CALLBACK_REGISTRATION {
                    Type: CF_CALLBACK_TYPE_CANCEL_FETCH_DATA,
                    Callback: Some(cf_cancel_fetch_callback),
                },
                CF_CALLBACK_REGISTRATION {
                    Type: CF_CALLBACK_TYPE_NONE,
                    Callback: None,
                },
            ];

            let connection_key = CfConnectSyncRoot(
                PCWSTR(sync_root_path.as_ptr()),
                callbacks.as_ptr(),
                None,
                CF_CONNECT_FLAG_NONE,
            )?;

            self.connection_key = Some(connection_key);
            self.callback_rx = Some(rx);

            tracing::info!("同步根已连接: {}", root_path.display());
        }

        Ok(())
    }

    /// 创建占位符文件
    pub fn create_placeholders(
        &self,
        base_dir: &Path,
        entries: &[PlaceholderEntry],
    ) -> anyhow::Result<u32> {
        if entries.is_empty() {
            return Ok(0);
        }

        unsafe {
            let base_path = to_pcwstr(base_dir);

            let mut placeholders: Vec<CF_PLACEHOLDER_CREATE_INFO> = entries
                .iter()
                .map(|entry| {
                    let relative_name_w = to_pcwstr_str(&entry.relative_path);
                    let file_attributes = if entry.is_dir {
                        FILE_ATTRIBUTE_DIRECTORY.0
                    } else {
                        FILE_ATTRIBUTE_NORMAL.0
                    };

                    CF_PLACEHOLDER_CREATE_INFO {
                        RelativeFileName: PCWSTR(relative_name_w.as_ptr()),
                        FsMetadata: CF_FS_METADATA {
                            BasicInfo: FILE_BASIC_INFO {
                                CreationTime: 0,
                                LastAccessTime: 0,
                                LastWriteTime: 0,
                                ChangeTime: 0,
                                FileAttributes: file_attributes,
                            },
                            FileSize: entry.file_size as i64,
                        },
                        FileIdentity: entry.file_identity.as_ptr() as *const std::os::raw::c_void,
                        FileIdentityLength: entry.file_identity.len() as u32,
                        Flags: CF_PLACEHOLDER_CREATE_FLAG_NONE,
                        Result: S_OK,
                        CreateUsn: 0,
                    }
                })
                .collect();

            let mut entries_processed: u32 = 0;

            CfCreatePlaceholders(
                PCWSTR(base_path.as_ptr()),
                &mut placeholders,
                CF_CREATE_FLAG_NONE,
                Some(&mut entries_processed),
            )?;

            tracing::debug!("创建占位符: {} 个", entries_processed);
            Ok(entries_processed)
        }
    }

    /// 水合文件
    pub fn hydrate_placeholder(&self, file_path: &Path) -> anyhow::Result<()> {
        unsafe {
            let file = std::fs::File::open(file_path)?;
            let handle = HANDLE(file.as_raw_handle() as *mut _);

            CfHydratePlaceholder(
                handle,
                0,
                -1i64,
                CF_HYDRATE_FLAG_NONE,
                None,
            )?;

            tracing::debug!("水合完成: {}", file_path.display());
            Ok(())
        }
    }

    /// 脱水文件
    pub fn dehydrate_placeholder(&self, file_path: &Path) -> anyhow::Result<()> {
        unsafe {
            let file = std::fs::File::open(file_path)?;
            let handle = HANDLE(file.as_raw_handle() as *mut _);

            CfDehydratePlaceholder(
                handle,
                0,
                -1i64,
                CF_DEHYDRATE_FLAG_NONE,
                None,
            )?;

            tracing::debug!("脱水完成: {}", file_path.display());
            Ok(())
        }
    }

    /// 断开同步根连接
    pub fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(key) = self.connection_key.take() {
            unsafe {
                CfDisconnectSyncRoot(key)?;
            }
            tracing::info!("同步根已断开");
        }
        self.callback_rx = None;
        Ok(())
    }
}

/// CFApi 回调：水合请求
unsafe extern "system" fn cf_fetch_data_callback(
    callback_info: *const CF_CALLBACK_INFO,
    _callback_parameters: *const CF_CALLBACK_PARAMETERS,
) {
    if callback_info.is_null() {
        return;
    }
    let info = &*callback_info;
    tracing::debug!(
        "CFApi 水合请求: file_identity_len={}",
        info.FileIdentityLength
    );
}

/// CFApi 回调：取消水合
unsafe extern "system" fn cf_cancel_fetch_callback(
    callback_info: *const CF_CALLBACK_INFO,
    _callback_parameters: *const CF_CALLBACK_PARAMETERS,
) {
    if callback_info.is_null() {
        return;
    }
    tracing::debug!("CFApi 取消水合请求");
}

/// Path → wide null-terminated Vec<u16>
fn to_pcwstr(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(path).encode_wide().collect();
    wide.push(0);
    wide
}

/// &str → wide null-terminated Vec<u16>
fn to_pcwstr_str(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide
}
