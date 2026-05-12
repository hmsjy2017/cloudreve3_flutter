import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:logger/logger.dart';

import '../../../data/models/sync_config_model.dart';
import '../../../data/models/sync_event_model.dart';
import '../../../data/models/sync_status_model.dart';
import '../../../services/sync_service.dart';

enum SyncState {
  idle,
  initializing,
  initialSync,
  continuous,
  paused,
  error,
  stopped,
}

class SyncProvider extends ChangeNotifier {
  final _log = Logger();

  SyncState _state = SyncState.idle;
  String? _errorMessage;
  SyncSummaryModel? _lastSummary;
  int _syncedFiles = 0;
  int _totalFiles = 0;
  int _conflictCount = 0;
  int _uploadingCount = 0;
  int _downloadingCount = 0;
  String? _currentFile;
  StreamSubscription<SyncEventModel>? _eventSub;

  SyncState get state => _state;
  String? get errorMessage => _errorMessage;
  SyncSummaryModel? get lastSummary => _lastSummary;
  int get syncedFiles => _syncedFiles;
  int get totalFiles => _totalFiles;
  int get conflictCount => _conflictCount;
  int get uploadingCount => _uploadingCount;
  int get downloadingCount => _downloadingCount;
  String? get currentFile => _currentFile;

  bool get isActive =>
      _state == SyncState.initializing ||
      _state == SyncState.initialSync ||
      _state == SyncState.continuous;

  bool get isPaused => _state == SyncState.paused;
  bool get hasError => _state == SyncState.error;

  double get progress =>
      _totalFiles > 0 ? _syncedFiles / _totalFiles : 0.0;

  /// 初始化并启动同步
  Future<void> startSync(SyncConfigModel config) async {
    _state = SyncState.initializing;
    _errorMessage = null;
    notifyListeners();

    try {
      await SyncService.instance.init(config);
      _subscribeEvents();

      _state = SyncState.initialSync;
      notifyListeners();

      final summary = await SyncService.instance.startInitialSync();
      _lastSummary = summary;
      _state = SyncState.continuous;
      notifyListeners();

      // 自动启动持续同步
      await SyncService.instance.startContinuousSync();
    } catch (e) {
      _state = SyncState.error;
      _errorMessage = e.toString();
      _log.e('同步启动失败: $e');
      notifyListeners();
    }
  }

  void _subscribeEvents() {
    _eventSub = SyncService.instance.events.listen((event) {
      switch (event) {
        case SyncStateChanged(:final newState):
          _state = _parseState(newState);
        case SyncProgress(:final synced, :final total, :final currentFile):
          _syncedFiles = synced;
          _totalFiles = total;
          _currentFile = currentFile;
        case SyncFileUploaded():
          _syncedFiles++;
        case SyncFileDownloaded():
          _syncedFiles++;
        case SyncConflictDetected():
          _conflictCount++;
        case SyncError(:final message, :final recoverable):
          if (!recoverable) {
            _state = SyncState.error;
            _errorMessage = message;
          }
        case SyncTokenExpired():
          _handleTokenExpired();
        case SyncInitialSyncComplete(:final summary):
          _lastSummary = summary;
          _state = SyncState.continuous;
          SyncService.instance.startContinuousSync();
        case SyncDiskSpaceWarning():
          _log.w('磁盘空间不足');
      }
      notifyListeners();
    });
  }

  SyncState _parseState(String state) {
    return switch (state) {
      'idle' => SyncState.idle,
      'initializing' => SyncState.initializing,
      'initialSync' => SyncState.initialSync,
      'continuous' => SyncState.continuous,
      'paused' => SyncState.paused,
      'error' => SyncState.error,
      'stopped' => SyncState.stopped,
      _ => SyncState.idle,
    };
  }

  void _handleTokenExpired() {
    _log.w('Token 过期，需要刷新');
    // TODO: 通过 Provider 作用域获取 AuthProvider 刷新 Token
  }

  /// 暂停同步
  Future<void> pause() async {
    await SyncService.instance.pause();
    _state = SyncState.paused;
    notifyListeners();
  }

  /// 恢复同步
  Future<void> resume() async {
    await SyncService.instance.resume();
    _state = SyncState.continuous;
    notifyListeners();
  }

  /// 停止同步
  Future<void> stop() async {
    await SyncService.instance.stop();
    _state = SyncState.stopped;
    notifyListeners();
  }

  /// 强制重新同步
  Future<void> forceSync() async {
    _state = SyncState.initialSync;
    notifyListeners();
    try {
      final summary = await SyncService.instance.forceSync();
      _lastSummary = summary;
      _state = SyncState.continuous;
    } catch (e) {
      _state = SyncState.error;
      _errorMessage = e.toString();
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _eventSub?.cancel();
    super.dispose();
  }
}
