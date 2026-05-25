import '../../src/rust/api/ffi_types.dart' as ffi;

class SyncStatusModel {
  final String state;
  final int syncedFiles;
  final int totalFiles;
  final int uploadingCount;
  final int downloadingCount;
  final int conflictCount;
  final int errorCount;
  final String? lastSyncTime;
  final String? errorMessage;

  const SyncStatusModel({
    this.state = 'idle',
    this.syncedFiles = 0,
    this.totalFiles = 0,
    this.uploadingCount = 0,
    this.downloadingCount = 0,
    this.conflictCount = 0,
    this.errorCount = 0,
    this.lastSyncTime,
    this.errorMessage,
  });

  bool get isIdle => state == 'idle';
  bool get isInitializing => state == 'initializing';
  bool get isInitialSync => state == 'initialSync';
  bool get isContinuous => state == 'continuous';
  bool get isPaused => state == 'paused';
  bool get isError => state == 'error';
  bool get isStopped => state == 'stopped';
  bool get isActive => isInitializing || isInitialSync || isContinuous;

  static SyncStatusModel fromFfi(ffi.SyncStatusFfi f) {
    return SyncStatusModel(
      state: f.state,
      syncedFiles: f.syncedFiles.toInt(),
      totalFiles: f.totalFiles.toInt(),
      uploadingCount: f.uploadingCount,
      downloadingCount: f.downloadingCount,
      conflictCount: f.conflictCount,
      errorCount: f.errorCount,
      lastSyncTime: f.lastSyncTime,
      errorMessage: f.errorMessage,
    );
  }
}
