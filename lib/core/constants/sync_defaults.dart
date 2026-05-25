import 'dart:io';

class SyncDefaults {
  SyncDefaults._();

  /// 默认同步目录
  static String defaultLocalRoot() {
    if (Platform.isWindows) {
      return '${Platform.environment['USERPROFILE']}\\Documents\\Cloudreve4';
    } else if (Platform.isLinux) {
      final xdgDownload =
          Platform.environment['XDG_DOWNLOAD_DIR'] ?? ("${Platform.environment['HOME'] ?? ''}/Downloads");
      return '$xdgDownload/Cloudreve4';
    } else if (Platform.isAndroid) {
      return ''; // Android 使用系统相册目录
    }
    return '';
  }

  static const String defaultRemoteRoot = 'cloudreve://my';
  static const String defaultSyncMode = 'full';
  static const String defaultConflictStrategy = 'keep_both';
  static const int defaultMaxConcurrentTransfers = 3;
  static const int defaultBandwidthLimitKbps = 0;
  static const int defaultMaxWorkers = 0; // 0 = CPU 核心数
  static const String defaultLogLevel = 'info';
}
