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

class SyncSummaryModel {
  final int uploaded;
  final int downloaded;
  final int conflicts;
  final int skipped;
  final int deletedLocal;
  final int deletedRemote;
  final int durationMs;

  const SyncSummaryModel({
    this.uploaded = 0,
    this.downloaded = 0,
    this.conflicts = 0,
    this.skipped = 0,
    this.deletedLocal = 0,
    this.deletedRemote = 0,
    this.durationMs = 0,
  });
}
