import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:file_picker/file_picker.dart';
import 'package:path_provider/path_provider.dart';
import 'package:open_file/open_file.dart';

import '../../../core/constants/sync_defaults.dart';
import '../../../data/models/sync_config_model.dart';
import '../../providers/auth_provider.dart';
import '../../providers/sync_provider.dart';
import '../../widgets/desktop_constrained.dart';
import '../../widgets/folder_picker.dart';
import '../../widgets/toast_helper.dart';
import 'sync_log_viewer_page.dart';

/// 同步设置页面
class SyncSettingsPage extends StatefulWidget {
  const SyncSettingsPage({super.key});

  @override
  State<SyncSettingsPage> createState() => _SyncSettingsPageState();
}

class _SyncSettingsPageState extends State<SyncSettingsPage> {
  late TextEditingController _localRootController;
  String _remoteRoot = SyncDefaults.defaultRemoteRoot;
  String _syncMode = 'full';
  String _conflictStrategy = 'keep_both';
  int _maxConcurrent = 3;
  int _bandwidthLimitKbps = 0;
  int _maxWorkers = SyncDefaults.defaultMaxWorkers;
  String _syncLogFilePath = '';
  int? _syncLogFileSize;

  @override
  void initState() {
    super.initState();
    _localRootController = TextEditingController(
      text: SyncDefaults.defaultLocalRoot(),
    );

    WidgetsBinding.instance.addPostFrameCallback((_) {
      final sync = context.read<SyncProvider>();
      final config = sync.persistedConfig;
      if (config != null) {
        setState(() {
          _localRootController.text = config.localRoot;
          _remoteRoot = config.remoteRoot;
          _syncMode = config.syncMode;
          _conflictStrategy = config.conflictStrategy;
          _maxConcurrent = config.maxConcurrentTransfers;
          _bandwidthLimitKbps = config.bandwidthLimitKbps;
          _maxWorkers = config.maxWorkers;
        });
      }
      _loadSyncLogInfo();
    });
  }

  @override
  void dispose() {
    _localRootController.dispose();
    super.dispose();
  }

  bool get _isDesktop => Platform.isWindows || Platform.isLinux;

  @override
  Widget build(BuildContext context) {
    final sync = context.watch<SyncProvider>();
    final auth = context.watch<AuthProvider>();

    return Scaffold(
      appBar: AppBar(title: const Text('文件同步')),
      body: DesktopConstrained(
        child: ListView(
          children: [
            _buildSyncStatus(sync),
            if (_isDesktop) ...[
              _buildSection(
                title: '同步目录',
                children: [
                  ListTile(
                    leading: const Icon(Icons.folder_outlined),
                    title: const Text('本地同步目录'),
                    subtitle: Text(_localRootController.text),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () => _pickLocalFolder(),
                  ),
                  ListTile(
                    leading: const Icon(Icons.cloud_outlined),
                    title: const Text('远程同步目录'),
                    subtitle: Text(_remoteRoot),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () => _pickRemoteFolder(),
                  ),
                ],
              ),
              _buildSection(
                title: '同步模式',
                children: [
                  RadioGroup<String>(
                    groupValue: _syncMode,
                    onChanged: (v) {
                      if (v != null) _handleSyncModeChange(v);
                    },
                    child: Column(
                      children: [
                        RadioListTile<String>(
                          title: const Text('全量同步'),
                          subtitle: const Text('双向同步所有文件'),
                          value: 'full',
                        ),
                        RadioListTile<String>(
                          title: const Text('仅上传本地到远程'),
                          subtitle: const Text('本地文件同步到远程，不影响远程已有文件'),
                          value: 'upload_only',
                        ),
                        RadioListTile<String>(
                          title: const Text('仅下载远程到本地'),
                          subtitle: const Text('远程文件同步到本地，本地修改不影响远程'),
                          value: 'download_only',
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              if (_syncMode == 'full')
                _buildSection(
                  title: '冲突处理',
                  children: [
                    ListTile(
                      leading: const Icon(Icons.merge_outlined),
                      title: const Text('冲突解决策略'),
                      subtitle: Text(_conflictStrategyLabel(_conflictStrategy)),
                      trailing: const Icon(Icons.chevron_right),
                      onTap: () => _pickConflictStrategy(),
                    ),
                  ],
                ),
            ],
            if (Platform.isAndroid)
              _buildSection(
                title: '相册同步',
                children: [
                  SwitchListTile(
                    title: const Text('自动备份相册'),
                    subtitle: const Text('将手机照片自动备份到云端'),
                    value: _syncMode == 'album',
                    onChanged: (v) {
                      setState(() => _syncMode = v ? 'album' : 'full');
                      _pushConfigIfActive();
                    },
                  ),
                ],
              ),
            _buildSection(
              title: '性能',
              children: [
                if (_isDesktop)
                  ListTile(
                    leading: const Icon(Icons.work_outline),
                    title: const Text('最大并发任务数'),
                    subtitle: Text(_maxWorkersLabel),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () => _pickMaxWorkers(),
                  ),
                ListTile(
                  leading: const Icon(Icons.sync_outlined),
                  title: const Text('最大并发传输数'),
                  subtitle: Text('$_maxConcurrent'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => _pickConcurrency(),
                ),
                ListTile(
                  leading: const Icon(Icons.speed_outlined),
                  title: const Text('带宽限制'),
                  subtitle: Text(
                    _bandwidthLimitKbps > 0
                        ? '${(_bandwidthLimitKbps / 1024).toStringAsFixed(1)} MB/s'
                        : '不限制',
                  ),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => _pickBandwidthLimit(),
                ),
              ],
            ),
            _buildSection(
              title: '同步日志',
              children: [
                ListTile(
                  title: const Text('日志文件路径'),
                  subtitle: Text(
                    _syncLogFilePath.isNotEmpty ? _syncLogFilePath : '未初始化',
                    style: const TextStyle(fontSize: 11),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                ListTile(
                  title: const Text('日志文件大小'),
                  subtitle: Text(_formatBytes(_syncLogFileSize)),
                ),
                if (!Platform.isAndroid)
                  ListTile(
                    leading: const Icon(Icons.folder_open),
                    title: const Text('打开日志目录'),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: _openSyncLogFolder,
                  ),
                ListTile(
                  leading: const Icon(Icons.file_download_outlined),
                  title: const Text('导出日志'),
                  subtitle: const Text('导出到 Download 目录'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: _exportSyncLog,
                ),
                ListTile(
                  leading: const Icon(Icons.visibility_outlined),
                  title: const Text('预览日志'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: _previewSyncLog,
                ),
                ListTile(
                  leading: const Icon(Icons.delete_outline, color: Colors.red),
                  title: const Text('清空日志'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: _clearSyncLog,
                ),
              ],
            ),
            if (!sync.isActive && !sync.isPaused)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
                child: FilledButton.icon(
                  onPressed: () => _startSync(auth, sync),
                  icon: const Icon(Icons.play_arrow),
                  label: const Text('开始同步'),
                ),
              ),
            if (sync.isActive || sync.isPaused)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                child: Row(
                  children: [
                    if (sync.isActive)
                      Expanded(
                        child: OutlinedButton.icon(
                          onPressed: () => sync.pause(),
                          icon: const Icon(Icons.pause),
                          label: const Text('暂停'),
                        ),
                      ),
                    if (sync.isPaused)
                      Expanded(
                        child: FilledButton.icon(
                          onPressed: () => sync.resume(),
                          icon: const Icon(Icons.play_arrow),
                          label: const Text('恢复'),
                        ),
                      ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () => _stopSync(sync),
                        icon: const Icon(Icons.stop),
                        label: const Text('停止'),
                        style: OutlinedButton.styleFrom(
                          foregroundColor: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            if (sync.isActive)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: OutlinedButton.icon(
                  onPressed: () => sync.forceSync(),
                  icon: const Icon(Icons.refresh),
                  label: const Text('强制重新同步'),
                ),
              ),
            const SizedBox(height: 32),
          ],
        ),
      ),
    );
  }

  String get _maxWorkersLabel {
    if (_maxWorkers == 0) return '自动 (CPU核心数)';
    return '$_maxWorkers';
  }

  Future<void> _handleSyncModeChange(String newMode) async {
    if (newMode == 'upload_only') {
      final confirmed = await _showModeConfirmDialog(
        title: '仅上传本地到远程',
        description: '此模式下：\n\n'
            '• 本地新增、修改的文件将上传到远程\n'
            '• 本地重命名、移动的文件会在远程同步操作\n'
            '• 本地删除的文件不会删除远程副本\n'
            '• 不会下载远程新增或修改的文件\n'
            '• 不监听远程事件（不消耗 SSE 连接）\n\n'
            '适用于只需要备份本地文件到云端的场景。',
      );
      if (confirmed != true) return;
    } else if (newMode == 'download_only') {
      final confirmed = await _showModeConfirmDialog(
        title: '仅下载远程到本地',
        description: '此模式下：\n\n'
            '• 远程新增、修改的文件将下载到本地\n'
            '• 远程删除的文件将同步删除本地副本\n'
            '• 远程重命名、移动会在本地同步\n'
            '• 本地的所有增删改操作不会影响远程\n'
            '• 本地删除的文件如果远程仍存在会重新下载\n'
            '• 不监听本地文件变化\n\n'
            '适用于只需要在本地保留远程副本的场景。',
      );
      if (confirmed != true) return;
    }

    setState(() => _syncMode = newMode);
    _pushConfigIfActive();
  }

  Future<bool?> _showModeConfirmDialog({
    required String title,
    required String description,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: Text(description),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('确认切换'),
          ),
        ],
      ),
    );
  }

  Widget _buildSyncStatus(SyncProvider sync) {
    return _buildSection(
      title: '同步状态',
      children: [
        ListTile(
          leading: Icon(
            _stateIcon(sync),
            color: _stateColor(sync),
          ),
          title: Text(_stateLabel(sync)),
          subtitle: sync.hasError && sync.errorMessage != null
              ? Text(
                  sync.errorMessage!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                )
              : null,
        ),
        if (sync.isActive || sync.isPaused) ...[
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: LinearProgressIndicator(
              value: sync.totalFiles > 0 ? sync.progress : null,
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Text(
              sync.totalFiles > 0
                  ? '${sync.syncedFiles} / ${sync.totalFiles} 文件'
                  : sync.state == SyncState.continuous
                      ? '持续同步中'
                      : '正在同步...',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ),
          if (sync.currentFile != null)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Text(
                sync.currentFile!,
                style: Theme.of(context).textTheme.bodySmall,
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
        ],
        if (sync.lastSummary != null) ...[
          const Divider(indent: 16, endIndent: 16),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Wrap(
              spacing: 16,
              children: [
                _summaryChip('上传', sync.lastSummary!.uploaded),
                _summaryChip('下载', sync.lastSummary!.downloaded),
                _summaryChip('冲突', sync.lastSummary!.conflicts),
                _summaryChip('失败', sync.lastSummary!.failed),
                _summaryChip('跳过', sync.lastSummary!.skipped),
                _summaryChip('删本地', sync.lastSummary!.deletedLocal),
                _summaryChip('删远程', sync.lastSummary!.deletedRemote),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _summaryChip(String label, int value) {
    return Chip(
      label: Text('$label: $value'),
      labelStyle: Theme.of(context).textTheme.bodySmall,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      visualDensity: VisualDensity.compact,
    );
  }

  IconData _stateIcon(SyncProvider sync) {
    if (sync.isActive) return Icons.sync;
    if (sync.isPaused) return Icons.pause_circle_outline;
    if (sync.hasError) return Icons.error_outline;
    if (sync.state == SyncState.stopped) return Icons.stop_circle_outlined;
    return Icons.cloud_off;
  }

  Color _stateColor(SyncProvider sync) {
    if (sync.isActive) return Theme.of(context).colorScheme.primary;
    if (sync.isPaused) return Colors.orange;
    if (sync.hasError) return Theme.of(context).colorScheme.error;
    return Theme.of(context).disabledColor;
  }

  String _stateLabel(SyncProvider sync) {
    return switch (sync.state) {
      SyncState.idle => '未启动',
      SyncState.initializing => '正在初始化...',
      SyncState.initialSync => '初始同步中',
      SyncState.continuous => '持续同步中',
      SyncState.paused => '已暂停',
      SyncState.error => '同步错误',
      SyncState.stopped => '已停止',
    };
  }

  String _conflictStrategyLabel(String strategy) {
    return switch (strategy) {
      'keep_local' => '保留本地版本',
      'keep_remote' => '保留远程版本',
      'keep_both' => '保留两份（重命名本地）',
      'newest_wins' => '最新修改优先',
      'largest_wins' => '最大文件优先',
      'manual' => '手动处理',
      _ => strategy,
    };
  }

  Widget _buildSection({
    required String title,
    required List<Widget> children,
  }) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Text(
              title,
              style: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.bold,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
          ),
          Card(
            margin: const EdgeInsets.symmetric(horizontal: 16),
            child: Column(children: children),
          ),
        ],
      ),
    );
  }

  Future<void> _pickLocalFolder() async {
    final result = await FilePicker.platform.getDirectoryPath(
      dialogTitle: '选择本地同步目录',
    );
    if (result != null) {
      setState(() => _localRootController.text = result);
    }
  }

  Future<void> _pickRemoteFolder() async {
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('选择远程同步目录'),
        content: SizedBox(
          width: 400,
          height: 500,
          child: FolderPicker(
            currentPath: _remoteRoot,
            onFolderSelected: (path) => Navigator.pop(ctx, path),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
        ],
      ),
    );
    if (result != null && result.isNotEmpty) {
      setState(() {
        _remoteRoot = result == '/'
            ? SyncDefaults.defaultRemoteRoot
            : '${SyncDefaults.defaultRemoteRoot}${result.startsWith('/') ? result : '/$result'}';
      });
    }
  }

  Future<void> _pickConflictStrategy() async {
    final strategies = [
      ('keep_both', '保留两份（重命名本地）'),
      ('keep_local', '保留本地版本'),
      ('keep_remote', '保留远程版本'),
      ('newest_wins', '最新修改优先'),
      ('largest_wins', '最大文件优先'),
      ('manual', '手动处理'),
    ];

    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('冲突解决策略'),
        children: strategies
            .map(
              (e) => SimpleDialogOption(
                onPressed: () => Navigator.pop(ctx, e.$1),
                child: Row(
                  children: [
                    Icon(
                      e.$1 == _conflictStrategy
                          ? Icons.radio_button_checked
                          : Icons.radio_button_unchecked,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text(e.$2),
                  ],
                ),
              ),
            )
            .toList(),
      ),
    );

    if (result != null) {
      setState(() => _conflictStrategy = result);
      _pushConfigIfActive();
    }
  }

  Future<void> _pickConcurrency() async {
    final values = [1, 2, 3, 5, 8];
    final result = await showDialog<int>(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('最大并发传输数'),
        children: values
            .map(
              (v) => SimpleDialogOption(
                onPressed: () => Navigator.pop(ctx, v),
                child: Row(
                  children: [
                    Icon(
                      v == _maxConcurrent
                          ? Icons.radio_button_checked
                          : Icons.radio_button_unchecked,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text('$v'),
                  ],
                ),
              ),
            )
            .toList(),
      ),
    );
    if (result != null) {
      setState(() => _maxConcurrent = result);
      _pushConfigIfActive();
    }
  }

  Future<void> _pickMaxWorkers() async {
    final controller = TextEditingController(text: _maxWorkers.toString());
    final result = await showDialog<int>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('最大并发任务数'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: controller,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                hintText: '0 = 自动 (CPU核心数)',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              '0 表示自动等于 CPU 核心数\n最大不超过 CPU 核心数的 2 倍，超出无效',
              style: Theme.of(ctx).textTheme.bodySmall?.copyWith(
                    color: Theme.of(ctx).hintColor,
                  ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () {
              final val = int.tryParse(controller.text) ?? 0;
              Navigator.pop(ctx, val.clamp(0, 999));
            },
            child: const Text('确定'),
          ),
        ],
      ),
    );
    if (result != null) {
      setState(() => _maxWorkers = result);
      _pushConfigIfActive();
    }
  }

  Future<void> _pickBandwidthLimit() async {
    final options = [
      (0, '不限制'),
      (1024, '1 MB/s'),
      (2048, '2 MB/s'),
      (5120, '5 MB/s'),
      (10240, '10 MB/s'),
    ];

    final result = await showDialog<int>(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('带宽限制'),
        children: options
            .map(
              (e) => SimpleDialogOption(
                onPressed: () => Navigator.pop(ctx, e.$1),
                child: Row(
                  children: [
                    Icon(
                      e.$1 == _bandwidthLimitKbps
                          ? Icons.radio_button_checked
                          : Icons.radio_button_unchecked,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text(e.$2),
                  ],
                ),
              ),
            )
            .toList(),
      ),
    );
    if (result != null) {
      setState(() => _bandwidthLimitKbps = result);
      _pushConfigIfActive();
    }
  }

  void _pushConfigIfActive() {
    final sync = context.read<SyncProvider>();
    if (!sync.isActive && !sync.isPaused) return;

    final config = sync.persistedConfig;
    if (config == null) return;

    final updated = config.copyWith(
      syncMode: _syncMode,
      conflictStrategy: _conflictStrategy,
      maxConcurrentTransfers: _maxConcurrent,
      bandwidthLimitKbps: _bandwidthLimitKbps,
      maxWorkers: _maxWorkers,
    );
    sync.updateConfig(updated);
  }

  Future<void> _startSync(AuthProvider auth, SyncProvider sync) async {
    final server = auth.currentServer;
    final token = auth.token;
    if (server == null || token == null) {
      ToastHelper.failure('请先登录');
      return;
    }

    final appSupportDir = await getApplicationSupportDirectory();

    final config = SyncConfigModel(
      baseUrl: server.baseUrl,
      accessToken: token.accessToken,
      refreshToken: token.refreshToken,
      localRoot: _localRootController.text,
      remoteRoot: _remoteRoot,
      syncMode: _syncMode,
      conflictStrategy: _conflictStrategy,
      maxConcurrentTransfers: _maxConcurrent,
      bandwidthLimitKbps: _bandwidthLimitKbps,
      maxWorkers: _maxWorkers,
      dataDir: appSupportDir.path,
      clientId: '',
    );

    await sync.startSync(config);
  }

  Future<void> _stopSync(SyncProvider sync) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('停止同步'),
        content: const Text('确定要停止文件同步吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(ctx).colorScheme.error,
            ),
            child: const Text('停止'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await sync.stop();
    }
  }

  // ===== 同步日志 =====

  Future<String> _getSyncLogPath() async {
    final appDir = await getApplicationSupportDirectory();
    return '${appDir.path}${Platform.pathSeparator}sync_core${Platform.pathSeparator}logs${Platform.pathSeparator}sync_log.txt';
  }

  Future<void> _loadSyncLogInfo() async {
    final path = await _getSyncLogPath();
    final file = File(path);
    int? size;
    if (await file.exists()) {
      size = await file.length();
    }
    if (mounted) {
      setState(() {
        _syncLogFilePath = path;
        _syncLogFileSize = size;
      });
    }
  }

  String _formatBytes(int? bytes) {
    if (bytes == null) return '未知';
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }

  Future<void> _openSyncLogFolder() async {
    try {
      if (_syncLogFilePath.isEmpty) {
        ToastHelper.error('日志文件路径未获取');
        return;
      }
      final dir = File(_syncLogFilePath).parent.path;
      final result = await OpenFile.open(dir);
      if (result.type != ResultType.done) {
        if (mounted) ToastHelper.error('无法打开目录：${result.message}');
      }
    } catch (e) {
      if (mounted) ToastHelper.error('打开目录失败：$e');
    }
  }

  Future<void> _exportSyncLog() async {
    try {
      final srcPath = await _getSyncLogPath();
      final srcFile = File(srcPath);
      if (!await srcFile.exists()) {
        if (mounted) ToastHelper.error('日志文件不存在');
        return;
      }
      final downloadDir = await getDownloadsDirectory();
      if (downloadDir == null) {
        if (mounted) ToastHelper.error('无法获取下载目录');
        return;
      }
      final timestamp = DateTime.now().toIso8601String().replaceAll(':', '-').substring(0, 19);
      final destPath = '${downloadDir.path}${Platform.pathSeparator}sync_core_log_$timestamp.txt';
      await srcFile.copy(destPath);
      if (mounted) ToastHelper.success('日志已导出到：$destPath');
    } catch (e) {
      if (mounted) ToastHelper.error('导出日志失败：$e');
    }
  }

  Future<void> _previewSyncLog() async {
    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => const SyncLogViewerPage(),
      ),
    );
  }

  Future<void> _clearSyncLog() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('清空同步日志'),
        content: const Text('确定要清空同步引擎日志文件内容吗？此操作不可恢复。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(backgroundColor: Theme.of(ctx).colorScheme.error),
            child: const Text('清空'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      final path = await _getSyncLogPath();
      final file = File(path);
      if (await file.exists()) {
        await file.writeAsString('');
      }
      await _loadSyncLogInfo();
      if (mounted) ToastHelper.success('同步日志已清空');
    }
  }
}
