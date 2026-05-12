/// Android 平台适配层
///
/// 提供相册同步功能，由 Dart 侧驱动扫描，
/// Rust 侧负责上传逻辑和状态管理

pub struct AndroidAdapter;

impl AndroidAdapter {
    pub fn new() -> Self {
        Self
    }
}
