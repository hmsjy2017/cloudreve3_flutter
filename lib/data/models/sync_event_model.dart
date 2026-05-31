import '../../src/rust/api/ffi_types.dart' as ffi;

sealed class SyncEventModel {}

class SyncStateChanged extends SyncEventModel {
  final String newState;
  SyncStateChanged(this.newState);
}

class SyncProgress extends SyncEventModel {
  final int synced;
  final int total;
  final String currentFile;
  SyncProgress(this.synced, this.total, this.currentFile);
}

class SyncFileUploaded extends SyncEventModel {
  final String localPath;
  final String remoteUri;
  SyncFileUploaded(this.localPath, this.remoteUri);
}

class SyncFileDownloaded extends SyncEventModel {
  final String localPath;
  final String remoteUri;
  SyncFileDownloaded(this.localPath, this.remoteUri);
}

class SyncConflictDetected extends SyncEventModel {
  final String localPath;
  final String conflictType;
  SyncConflictDetected(this.localPath, this.conflictType);
}

class SyncError extends SyncEventModel {
  final String message;
  final bool recoverable;
  SyncError(this.message, this.recoverable);
}

class SyncTokenExpired extends SyncEventModel {}

class SyncDiskSpaceWarning extends SyncEventModel {
  final int availableMb;
  SyncDiskSpaceWarning(this.availableMb);
}

class SyncInitialSyncComplete extends SyncEventModel {
  final SyncSummaryModel summary;
  SyncInitialSyncComplete(this.summary);
}

class SyncWorkerCompleted extends SyncEventModel {
  final String taskId;
  final int uploaded;
  final int downloaded;
  final int renamed;
  final int moved;
  final int failed;
  final int durationMs;
  SyncWorkerCompleted({
    required this.taskId,
    required this.uploaded,
    required this.downloaded,
    required this.renamed,
    required this.moved,
    required this.failed,
    required this.durationMs,
  });
}

class SyncWorkerFailed extends SyncEventModel {
  final String taskId;
  final String message;
  SyncWorkerFailed({required this.taskId, required this.message});
}

class SyncTaskItemUpdated extends SyncEventModel {
  final String taskId;
  final String relativePath;
  final String action;
  final String status;
  SyncTaskItemUpdated({
    required this.taskId,
    required this.relativePath,
    required this.action,
    required this.status,
  });
}

/// 将 FFI 事件转换为 Dart 模型
SyncEventModel? syncEventFromFfi(ffi.SyncEventFfi event) {
  return event.when(
    stateChanged: (newState) => SyncStateChanged(newState),
    progress: (synced, total, currentFile) =>
        SyncProgress(synced.toInt(), total.toInt(), currentFile),
    fileUploaded: (localPath, remoteUri) =>
        SyncFileUploaded(localPath, remoteUri),
    fileDownloaded: (localPath, remoteUri) =>
        SyncFileDownloaded(localPath, remoteUri),
    conflictDetected: (localPath, conflictType) =>
        SyncConflictDetected(localPath, conflictType),
    error: (message, recoverable) => SyncError(message, recoverable),
    tokenExpired: () => SyncTokenExpired(),
    diskSpaceWarning: (availableMb) =>
        SyncDiskSpaceWarning(availableMb.toInt()),
    initialSyncComplete: (summary) =>
        SyncInitialSyncComplete(SyncSummaryModel.fromFfi(summary)),
    workerStarted: (taskId, trigger, uploadCount, downloadCount) => null,
    workerCompleted: (taskId, uploaded, downloaded, renamed, moved, failed,
            durationMs) =>
        SyncWorkerCompleted(
          taskId: taskId,
          uploaded: uploaded,
          downloaded: downloaded,
          renamed: renamed,
          moved: moved,
          failed: failed,
          durationMs: durationMs.toInt(),
        ),
    workerFailed: (taskId, message) =>
        SyncWorkerFailed(taskId: taskId, message: message),
    taskItemUpdated: (taskId, relativePath, action, status) =>
        SyncTaskItemUpdated(taskId: taskId, relativePath: relativePath, action: action, status: status),
  );
}

class SyncSummaryModel {
  final int uploaded;
  final int downloaded;
  final int renamed;
  final int moved;
  final int conflicts;
  final int failed;
  final int skipped;
  final int deletedLocal;
  final int deletedRemote;
  final int durationMs;

  const SyncSummaryModel({
    this.uploaded = 0,
    this.downloaded = 0,
    this.renamed = 0,
    this.moved = 0,
    this.conflicts = 0,
    this.failed = 0,
    this.skipped = 0,
    this.deletedLocal = 0,
    this.deletedRemote = 0,
    this.durationMs = 0,
  });

  static SyncSummaryModel fromFfi(ffi.SyncSummaryFfi f) {
    return SyncSummaryModel(
      uploaded: f.uploaded,
      downloaded: f.downloaded,
      renamed: f.renamed,
      moved: f.moved,
      conflicts: f.conflicts,
      failed: f.failed,
      skipped: f.skipped,
      deletedLocal: f.deletedLocal,
      deletedRemote: f.deletedRemote,
      durationMs: f.durationMs.toInt(),
    );
  }
}
