import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:logger/logger.dart';

import '../../../data/models/sync_config_model.dart';
import '../../../data/models/sync_event_model.dart';
import '../../../data/models/sync_task_model.dart';
import '../../../services/storage_service.dart';
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
  Timer? _pollTimer;
  int _pollIntervalSeconds = 1; // 动态调节：活跃1s，空闲3s

  // 任务列表
  List<SyncTaskModel> _activeTasks = [];
  List<SyncTaskModel> _recentTasks = [];
  int _activeWorkerCount = 0;
  // 任务详情缓存: taskId -> items
  final Map<String, List<SyncTaskItemModel>> _taskDetailCache = {};
  bool _recentTasksLoaded = false;

  // 持久化的同步配置
  SyncConfigModel? _persistedConfig;

  SyncState get state => _state;
  String? get errorMessage => _errorMessage;
  SyncSummaryModel? get lastSummary => _lastSummary;
  int get syncedFiles => _syncedFiles;
  int get totalFiles => _totalFiles;
  int get conflictCount => _conflictCount;
  int get uploadingCount => _uploadingCount;
  int get downloadingCount => _downloadingCount;
  String? get currentFile => _currentFile;
  SyncConfigModel? get persistedConfig => _persistedConfig;
  List<SyncTaskModel> get activeTasks => _activeTasks;
  List<SyncTaskModel> get recentTasks => _recentTasks;
  int get activeWorkerCount => _activeWorkerCount;

  bool get isActive =>
      _state == SyncState.initializing ||
      _state == SyncState.initialSync ||
      _state == SyncState.continuous;

  bool get isPaused => _state == SyncState.paused;

  bool get hasError => _state == SyncState.error;

  double get progress =>
      _totalFiles > 0 ? _syncedFiles / _totalFiles : 0.0;

  /// 从持久化存储恢复同步配置和状态
  Future<void> restoreFromStorage() async {
    final configMap = await StorageService.instance.getSyncConfig();
    if (configMap != null) {
      try {
        _persistedConfig = SyncConfigModel(
          baseUrl: configMap['baseUrl'] as String? ?? '',
          accessToken: configMap['accessToken'] as String? ?? '',
          refreshToken: configMap['refreshToken'] as String? ?? '',
          localRoot: configMap['localRoot'] as String? ?? '',
          remoteRoot: configMap['remoteRoot'] as String? ?? 'cloudreve://my',
          syncMode: configMap['syncMode'] as String? ?? 'full',
          conflictStrategy: configMap['conflictStrategy'] as String? ?? 'keep_both',
          maxConcurrentTransfers: configMap['maxConcurrentTransfers'] as int? ?? 3,
          bandwidthLimitKbps: configMap['bandwidthLimitKbps'] as int? ?? 0,
          maxWorkers: configMap['maxWorkers'] as int? ?? 0,
          dataDir: configMap['dataDir'] as String? ?? '',
          clientId: configMap['clientId'] as String? ?? '',
        );
        _log.i('恢复同步配置: 模式=${_persistedConfig!.syncMode}, 冲突=${_persistedConfig!.conflictStrategy}, 并发=${_persistedConfig!.maxConcurrentTransfers}, 带宽=${_persistedConfig!.bandwidthLimitKbps}kbps, maxWorkers=${_persistedConfig!.maxWorkers}');
      } catch (e) {
        _log.e('恢复同步配置失败: $e');
      }
    }

    final savedState = await StorageService.instance.getSyncState();
    if (savedState != null && savedState != 'idle' && savedState != 'stopped') {
      _log.i('恢复同步状态: $savedState');
    }
  }

  /// 保存同步配置到持久化存储
  Future<void> _persistConfig(SyncConfigModel config) async {
    _persistedConfig = config;
    await StorageService.instance.setSyncConfig({
      'baseUrl': config.baseUrl,
      'accessToken': config.accessToken,
      'refreshToken': config.refreshToken,
      'localRoot': config.localRoot,
      'remoteRoot': config.remoteRoot,
      'syncMode': config.syncMode,
      'conflictStrategy': config.conflictStrategy,
      'maxConcurrentTransfers': config.maxConcurrentTransfers,
      'bandwidthLimitKbps': config.bandwidthLimitKbps,
      'maxWorkers': config.maxWorkers,
      'dataDir': config.dataDir,
      'clientId': config.clientId,
    });
  }

  /// 持久化同步状态
  Future<void> _persistState(SyncState state) async {
    final stateStr = switch (state) {
      SyncState.idle => 'idle',
      SyncState.initializing => 'initializing',
      SyncState.initialSync => 'initialSync',
      SyncState.continuous => 'continuous',
      SyncState.paused => 'paused',
      SyncState.error => 'error',
      SyncState.stopped => 'stopped',
    };
    await StorageService.instance.setSyncState(stateStr);
  }

  /// 初始化并启动同步
  Future<void> startSync(SyncConfigModel config) async {
    _state = SyncState.initializing;
    _errorMessage = null;
    _syncedFiles = 0;
    _totalFiles = 0;
    _currentFile = null;
    notifyListeners();

    // 确保 clientId 已注入（Dart 生成、持久化、全层共享）
    final clientId = await StorageService.instance.getOrCreateClientId();
    final configWithClientId = config.copyWith(clientId: clientId);

    // 持久化配置和状态
    await _persistConfig(configWithClientId);
    await _persistState(SyncState.initializing);

    try {
      await SyncService.instance.init(configWithClientId);
      _subscribeEvents();

      _state = SyncState.initialSync;
      await _persistState(SyncState.initialSync);
      notifyListeners();

      // 启动状态轮询
      _startPolling();

      // 启动初始同步（后台运行）
      SyncService.instance.startInitialSync().then((summary) async {
        _lastSummary = summary;
        _state = SyncState.continuous;
        await _persistState(SyncState.continuous);
        _log.i('初始同步完成');
        notifyListeners();

        // 自动启动持续同步
        SyncService.instance.startContinuousSync();
      }).catchError((e) async {
        _state = SyncState.error;
        _errorMessage = e.toString();
        await _persistState(SyncState.error);
        _log.e('初始同步失败: $e');
        notifyListeners();
      });
    } catch (e) {
      _state = SyncState.error;
      _errorMessage = e.toString();
      await _persistState(SyncState.error);
      _log.e('同步启动失败: $e');
      notifyListeners();
    }
  }

  /// 自动恢复同步（启动时调用，如果之前处于同步状态则恢复）
  Future<void> autoResumeIfNeeded({
    String? currentAccessToken,
    String? currentRefreshToken,
  }) async {
    if (_persistedConfig == null) {
      await restoreFromStorage();
    }
    if (_persistedConfig == null) return;

    final savedState = await StorageService.instance.getSyncState();
    if (savedState == null || savedState == 'idle' || savedState == 'stopped' || savedState == 'error') {
      return;
    }

    _log.i('自动恢复同步，上次状态: $savedState');

    var config = _persistedConfig!;
    if (currentAccessToken != null && currentRefreshToken != null) {
      config = config.copyWith(
        accessToken: currentAccessToken,
        refreshToken: currentRefreshToken,
      );
    }

    await startSync(config);
  }

  /// 热更新配置（同步运行中修改配置，无需重启引擎）
  Future<void> updateConfig(SyncConfigModel config) async {
    await _persistConfig(config);

    if (isActive || isPaused) {
      try {
        await SyncService.instance.updateConfig(config);
        _log.i('同步配置已热更新到引擎');
      } catch (e) {
        _log.e('热更新配置失败: $e');
      }
    }
  }

  /// 轮询 Rust 引擎状态（动态间隔：活跃1s，空闲3s）
  void _startPolling() {
    _pollTimer?.cancel();
    _pollIntervalSeconds = isActive || isPaused ? 1 : 3;
    _pollTimer = Timer.periodic(
      Duration(seconds: _pollIntervalSeconds),
      (_) => _pollStatus(),
    );
  }

  void _adjustPollInterval() {
    final target = isActive || isPaused ? 1 : 3;
    if (target != _pollIntervalSeconds) {
      _pollIntervalSeconds = target;
      _startPolling();
    }
  }

  void _stopPolling() {
    _pollTimer?.cancel();
    _pollTimer = null;
  }

  Future<void> _pollStatus() async {
    try {
      final status = await SyncService.instance.getStatus();
      _syncedFiles = status.syncedFiles;
      _totalFiles = status.totalFiles;
      _uploadingCount = status.uploadingCount;
      _downloadingCount = status.downloadingCount;
      _conflictCount = status.conflictCount;

      final rustState = _parseState(status.state);
      if (rustState != _state && rustState != SyncState.idle) {
        _state = rustState;
        await _persistState(rustState);
      }
      if (status.errorMessage != null && status.errorMessage!.isNotEmpty) {
        _errorMessage = status.errorMessage;
      }

      // 轮询活跃任务 + 刷新已完成任务
      try {
        final newActiveWorkerCount = await SyncService.instance.getActiveWorkerCount();
        final newActiveTasks = await SyncService.instance.getActiveTasksTyped();

        bool changed = newActiveWorkerCount != _activeWorkerCount;
        _activeWorkerCount = newActiveWorkerCount;

        // 活跃任务列表有变化时才更新
        if (!_taskListEqual(newActiveTasks, _activeTasks)) {
          _activeTasks = newActiveTasks;
          changed = true;
          // 清除不再活跃的任务的详情缓存
          _taskDetailCache.removeWhere(
            (key, _) => !newActiveTasks.any((t) => t.id == key),
          );
        }

        // 同步运行中：每次轮询刷新已完成任务（快任务可能在两次轮询间完成）
        if (_recentTasksLoaded && isActive) {
          await _refreshRecentTasks();
        }
        // Worker 从有变无时也刷新（覆盖停止/错误等非活跃场景）
        if (_activeWorkerCount == 0 && _recentTasksLoaded) {
          await _refreshRecentTasks();
        }

        if (changed) notifyListeners();
        else notifyListeners();
      } catch (_) {}

      notifyListeners();
      _adjustPollInterval();
    } catch (_) {}
  }

  /// 首次加载已完成任务
  Future<void> loadRecentTasks() async {
    if (_recentTasksLoaded) return;
    await _refreshRecentTasks();
  }

  Future<void> _refreshRecentTasks() async {
    try {
      final tasks = await SyncService.instance.getRecentTasksTyped();
      if (!_taskListEqual(tasks, _recentTasks)) {
        _recentTasks = tasks;
      }
      _recentTasksLoaded = true;
    } catch (_) {}
  }

  bool _taskListEqual(List<SyncTaskModel> a, List<SyncTaskModel> b) {
    if (a.length != b.length) return false;
    for (int i = 0; i < a.length; i++) {
      if (a[i].id != b[i].id ||
          a[i].completedCount != b[i].completedCount ||
          a[i].failedCount != b[i].failedCount ||
          a[i].status != b[i].status) {
        return false;
      }
    }
    return true;
  }

  void _subscribeEvents() {
    _eventSub = SyncService.instance.events.listen((event) async {
      switch (event) {
        case SyncStateChanged(:final newState):
          _state = _parseState(newState);
          await _persistState(_state);
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
            await _persistState(SyncState.error);
          }
        case SyncTokenExpired():
          _handleTokenExpired();
        case SyncInitialSyncComplete(:final summary):
          _lastSummary = summary;
          _state = SyncState.continuous;
          await _persistState(SyncState.continuous);
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
  }

  /// 获取任务详情（带缓存）
  Future<List<SyncTaskItemModel>> getTaskDetail(String taskId) async {
    if (_taskDetailCache.containsKey(taskId)) {
      return _taskDetailCache[taskId]!;
    }
    final items = await SyncService.instance.getTaskDetailTyped(taskId);
    _taskDetailCache[taskId] = items;
    notifyListeners();
    return items;
  }

  /// 读取缓存的任务详情（同步，无则返回 null）
  List<SyncTaskItemModel>? getCachedTaskDetail(String taskId) {
    return _taskDetailCache[taskId];
  }

  /// 清除任务详情缓存，下次获取时重新请求
  void invalidateTaskDetail(String taskId) {
    _taskDetailCache.remove(taskId);
  }

  /// 清除所有任务详情缓存
  void invalidateAllTaskDetails() {
    _taskDetailCache.clear();
    _recentTasksLoaded = false;
  }

  /// 暂停同步
  Future<void> pause() async {
    await SyncService.instance.pause();
    _state = SyncState.paused;
    await _persistState(SyncState.paused);
    notifyListeners();
  }

  /// 恢复同步
  Future<void> resume() async {
    await SyncService.instance.resume();
    _state = SyncState.continuous;
    await _persistState(SyncState.continuous);
    notifyListeners();
  }

  /// 停止同步
  Future<void> stop() async {
    _stopPolling();
    await SyncService.instance.stop();
    _state = SyncState.stopped;
    await _persistState(SyncState.stopped);
    notifyListeners();
  }

  /// 强制重新同步
  Future<void> forceSync() async {
    _state = SyncState.initialSync;
    _syncedFiles = 0;
    _totalFiles = 0;
    await _persistState(SyncState.initialSync);
    notifyListeners();

    try {
      final summary = await SyncService.instance.forceSync();
      _lastSummary = summary;
      _state = SyncState.continuous;
      await _persistState(SyncState.continuous);
    } catch (e) {
      _state = SyncState.error;
      _errorMessage = e.toString();
      await _persistState(SyncState.error);
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _stopPolling();
    _eventSub?.cancel();
    super.dispose();
  }
}
