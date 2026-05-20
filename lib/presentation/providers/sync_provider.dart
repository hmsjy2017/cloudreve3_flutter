import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:logger/logger.dart';

import '../../../data/models/sync_config_model.dart';
import '../../../data/models/sync_event_model.dart';
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
          dataDir: configMap['dataDir'] as String? ?? '',
          clientId: configMap['clientId'] as String? ?? '',
        );
        _log.i('恢复同步配置: 模式=${_persistedConfig!.syncMode}, 冲突=${_persistedConfig!.conflictStrategy}, 并发=${_persistedConfig!.maxConcurrentTransfers}, 带宽=${_persistedConfig!.bandwidthLimitKbps}kbps');
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
  /// [currentAccessToken] 和 [currentRefreshToken] 用于更新过期的 token
  Future<void> autoResumeIfNeeded({
    String? currentAccessToken,
    String? currentRefreshToken,
  }) async {
    // 确保持久化配置已加载
    if (_persistedConfig == null) {
      await restoreFromStorage();
    }
    if (_persistedConfig == null) return;

    final savedState = await StorageService.instance.getSyncState();
    if (savedState == null || savedState == 'idle' || savedState == 'stopped' || savedState == 'error') {
      return;
    }

    _log.i('自动恢复同步，上次状态: $savedState');

    // 用最新的 token 更新配置
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
    // 持久化新配置
    await _persistConfig(config);

    // 如果引擎正在运行，推送到 Rust
    if (isActive || isPaused) {
      try {
        await SyncService.instance.updateConfig(config);
        _log.i('同步配置已热更新到引擎');
      } catch (e) {
        _log.e('热更新配置失败: $e');
      }
    }
  }

  /// 轮询 Rust 引擎状态
  void _startPolling() {
    _pollTimer?.cancel();
    _pollTimer = Timer.periodic(
      const Duration(seconds: 2),
      (_) => _pollStatus(),
    );
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

      // 仅当 Rust 端状态与当前不一致时更新
      final rustState = _parseState(status.state);
      if (rustState != _state && rustState != SyncState.idle) {
        _state = rustState;
        await _persistState(rustState);
      }
      if (status.errorMessage != null && status.errorMessage!.isNotEmpty) {
        _errorMessage = status.errorMessage;
      }

      notifyListeners();
    } catch (_) {
      // 轮询失败不影响 UI
    }
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
    // TODO: 通过 Provider 作用域获取 AuthProvider 刷新 Token
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
