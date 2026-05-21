import 'dart:async';
import 'package:logger/logger.dart';

import '../data/models/sync_config_model.dart';
import '../data/models/sync_event_model.dart';
import '../data/models/sync_status_model.dart';
import '../data/models/sync_task_model.dart';
import '../src/rust/api/ffi.dart' as ffi;

/// 同步服务单例 - 桥接 Flutter UI 和 Rust 同步引擎
class SyncService {
  static final SyncService instance = SyncService._();
  SyncService._();

  final _log = Logger();
  bool _initialized = false;

  /// 事件流，供 SyncProvider 订阅
  final _eventController = StreamController<SyncEventModel>.broadcast();
  Stream<SyncEventModel> get events => _eventController.stream;

  /// 初始化同步引擎
  Future<void> init(SyncConfigModel config) async {
    if (_initialized) return;

    _log.d('初始化同步引擎: ${config.localRoot}');

    await ffi.initSyncEngine(config: config.toFfi());

    _initialized = true;
  }

  /// 执行初始全量同步
  Future<SyncSummaryModel> startInitialSync() async {
    _log.d('启动初始同步');
    final summary = await ffi.startInitialSync();
    return SyncSummaryModel.fromFfi(summary);
  }

  /// 启动持续同步
  Future<void> startContinuousSync() async {
    _log.d('启动持续同步');
    await ffi.startContinuousSync();
  }

  /// 停止同步
  Future<void> stop() async {
    _log.d('停止同步');
    await ffi.stopSync();
  }

  /// 暂停同步
  Future<void> pause() async {
    _log.d('暂停同步');
    await ffi.pauseSync();
  }

  /// 恢复同步
  Future<void> resume() async {
    _log.d('恢复同步');
    await ffi.resumeSync();
  }

  /// 强制重新同步
  Future<SyncSummaryModel> forceSync() async {
    _log.d('强制重新同步');
    final summary = await ffi.forceSync();
    return SyncSummaryModel.fromFfi(summary);
  }

  /// Token 变更时推送给 Rust
  Future<void> updateTokens(String accessToken) async {
    _log.d('更新 Token');
    await ffi.updateTokens(accessToken: accessToken);
  }

  /// 热更新同步配置（不重启引擎）
  Future<void> updateConfig(SyncConfigModel config) async {
    _log.d('热更新同步配置');
    await ffi.updateSyncConfig(config: config.toFfi());
  }

  /// 获取同步状态快照
  Future<SyncStatusModel> getStatus() async {
    final status = await ffi.getSyncStatus();
    return SyncStatusModel.fromFfi(status);
  }

  /// 获取活跃 Worker 数量
  Future<int> getActiveWorkerCount() async {
    return await ffi.getActiveWorkerCount();
  }

  /// 获取活跃的同步任务列表（typed）
  Future<List<SyncTaskModel>> getActiveTasksTyped() async {
    final tasks = await ffi.getActiveTasks();
    return tasks.map((t) => SyncTaskModel(
      id: t.id,
      trigger: t.trigger,
      totalCount: t.totalCount,
      completedCount: t.completedCount,
      failedCount: t.failedCount,
      status: t.status,
      createdAt: t.createdAt,
      updatedAt: t.updatedAt,
      finishedAt: t.finishedAt,
    )).toList();
  }

  /// 获取最近同步任务列表（typed）
  Future<List<SyncTaskModel>> getRecentTasksTyped({int limit = 20}) async {
    final tasks = await ffi.getRecentTasks(limit: limit);
    return tasks.map((t) => SyncTaskModel(
      id: t.id,
      trigger: t.trigger,
      totalCount: t.totalCount,
      completedCount: t.completedCount,
      failedCount: t.failedCount,
      status: t.status,
      createdAt: t.createdAt,
      updatedAt: t.updatedAt,
      finishedAt: t.finishedAt,
    )).toList();
  }

  /// 获取任务详情（typed）
  Future<List<SyncTaskItemModel>> getTaskDetailTyped(String taskId) async {
    final items = await ffi.getTaskDetail(taskId: taskId);
    return items.map((i) => SyncTaskItemModel(
      id: i.id.toInt(),
      taskId: i.taskId,
      relativePath: i.relativePath,
      actionType: i.actionType,
      status: i.status,
      fileSize: i.fileSize.toInt(),
      errorMessage: i.errorMessage,
      createdAt: i.createdAt,
      updatedAt: i.updatedAt,
    )).toList();
  }

  /// 水合文件 (Windows CFAPi)
  Future<void> hydrateFile(String localPath) async {
    _log.d('水合文件: $localPath');
    await ffi.hydrateFile(localPath: localPath);
  }

  /// 检查云端相册目录 (Android)
  Future<Map<String, dynamic>> checkCloudAlbumDirs(String baseUri) async {
    final result = await ffi.checkCloudAlbumDirs(baseUri: baseUri);
    return {
      'dcimExists': result.dcimExists,
      'picturesExists': result.picturesExists,
      'dcimUri': result.dcimUri,
      'picturesUri': result.picturesUri,
    };
  }

  /// 创建云端相册目录 (Android)
  Future<void> createCloudAlbumDirs(String baseUri) async {
    _log.d('创建云端相册目录: $baseUri');
    await ffi.createCloudAlbumDirs(baseUri: baseUri);
  }

  /// 销毁同步引擎
  Future<void> dispose() async {
    _log.d('销毁同步引擎');
    await _eventController.close();
    await ffi.disposeSyncEngine();
    _initialized = false;
  }
}
