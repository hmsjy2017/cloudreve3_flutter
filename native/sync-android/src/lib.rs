#![cfg(target_os = "android")]

//! Android 平台适配层
//!
//! 提供相册同步辅助功能：
//! - 文件类型过滤（仅图片/视频）
//! - 路径验证
//!
//! 实际的扫描由 Dart 侧通过平台通道完成，
//! 上传和状态管理由 sync-core 的 sync_album 处理

use std::path::Path;

/// 图片扩展名
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "raw", "tiff", "svg",
];

/// 视频扩展名
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "3gp", "webm",
];

pub struct AndroidAdapter;

impl Default for AndroidAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 判断文件是否为媒体文件（图片或视频）
    pub fn is_media_file(path: &str) -> bool {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        IMAGE_EXTENSIONS.contains(&ext.as_str()) || VIDEO_EXTENSIONS.contains(&ext.as_str())
    }

    /// 判断文件是否为图片
    pub fn is_image_file(path: &str) -> bool {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        IMAGE_EXTENSIONS.contains(&ext.as_str())
    }

    /// 判断文件是否为视频
    pub fn is_video_file(path: &str) -> bool {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        VIDEO_EXTENSIONS.contains(&ext.as_str())
    }

    /// 从路径列表中过滤出媒体文件
    pub fn filter_media_files(paths: &[String]) -> Vec<String> {
        paths.iter()
            .filter(|p| Self::is_media_file(p))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_media_file() {
        assert!(AndroidAdapter::is_image_file("/sdcard/DCIM/IMG_001.jpg"));
        assert!(AndroidAdapter::is_image_file("/sdcard/DCIM/IMG_001.JPEG"));
        assert!(AndroidAdapter::is_video_file("/sdcard/DCIM/VID_001.mp4"));
        assert!(!AndroidAdapter::is_media_file("/sdcard/Download/doc.pdf"));
        assert!(AndroidAdapter::is_media_file("/sdcard/Pictures/photo.webp"));
        assert!(AndroidAdapter::is_media_file("/sdcard/DCIM/photo.heic"));
    }

    #[test]
    fn test_filter_media_files() {
        let paths = vec![
            "/sdcard/DCIM/IMG_001.jpg".to_string(),
            "/sdcard/Download/doc.pdf".to_string(),
            "/sdcard/DCIM/VID_001.mp4".to_string(),
            "/sdcard/Music/song.mp3".to_string(),
        ];
        let filtered = AndroidAdapter::filter_media_files(&paths);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"/sdcard/DCIM/IMG_001.jpg".to_string()));
        assert!(filtered.contains(&"/sdcard/DCIM/VID_001.mp4".to_string()));
    }
}
