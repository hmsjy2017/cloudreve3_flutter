import 'dart:io';
import 'package:external_path/external_path.dart';

class SyncDefaults {
  SyncDefaults._();

  /// 默认同步目录（同步版本，Android 返回空串，需用 getDefaultLocalRoot 异步获取）
  static String defaultLocalRoot() {
    if (Platform.isWindows) {
      return '${Platform.environment['USERPROFILE']}\\Documents\\Cloudreve4';
    } else if (Platform.isLinux) {
      final xdgDownload =
          Platform.environment['XDG_DOWNLOAD_DIR'] ?? ("${Platform.environment['HOME'] ?? ''}/Downloads");
      return '$xdgDownload/Cloudreve4';
    } else if (Platform.isAndroid) {
      return '';
    }
    return '';
  }

  /// 默认同步目录（异步版本，Android 通过 ExternalPath 获取公共目录）
  static Future<String> getDefaultLocalRoot() async {
    if (Platform.isAndroid) {
      return getDefaultAndroidLocalRoot();
    }
    return defaultLocalRoot();
  }

  static const String defaultRemoteRoot = 'cloudreve://my';
  static const String defaultSyncMode = 'full';
  static const String defaultConflictStrategy = 'keep_both';
  static const int defaultMaxConcurrentTransfers = 3;
  static const int defaultBandwidthLimitKbps = 0;
  static const int defaultMaxWorkers = 0; // 0 = CPU 核心数
  static const String defaultLogLevel = 'info';

  // ===== Android 相册同步专用 =====
  static String get defaultAndroidLocalRoot {
    // runtime 获取 DCIM 公共目录，再拼接 /Camera
    // ExternalPath 无法 const，使用 getter 延迟求值
    throw UnsupportedError('Use getDefaultAndroidLocalRoot() instead');
  }

  static Future<String> getDefaultAndroidLocalRoot() async {
    final dcim = await ExternalPath.getExternalStoragePublicDirectory(
      ExternalPath.DIRECTORY_DCIM,
    );
    return '$dcim/Camera';
  }

  static const String defaultAndroidRemoteRoot = 'cloudreve://my/DCIM/Camera';
  static const String defaultAndroidSyncMode = 'album_upload';
}
