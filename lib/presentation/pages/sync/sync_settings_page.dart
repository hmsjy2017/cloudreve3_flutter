import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:file_picker/file_picker.dart';
import 'package:path_provider/path_provider.dart';

import '../../../core/constants/sync_defaults.dart';
import '../../../data/models/sync_config_model.dart';
import '../../providers/auth_provider.dart';
import '../../providers/sync_provider.dart';
import '../../widgets/desktop_constrained.dart';
import '../../widgets/toast_helper.dart';

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

  @override
  void initState() {
    super.initState();
    _localRootController = TextEditingController(
      text: SyncDefaults.defaultLocalRoot(),
    );

    // 从持久化配置恢复 UI 状态
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
        });
      }
    });
  }

  @override
  void dispose() {
    _localRootController.dispose();
    super.dispose();
  }

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
                  onTap: () => _editRemoteRoot(),
                ),
              ],
            ),
            _buildSection(
              title: '同步模式',
              children: [
                RadioGroup<String>(
                  groupValue: _syncMode,
                  onChanged: (v) { if (v != null) { setState(() => _syncMode = v); _pushConfigIfActive(); } },
                  child: Column(
                    children: [
                      RadioListTile<String>(
                        title: const Text('全量同步'),
                        subtitle: const Text('双向同步所有文件'),
                        value: 'full',
                      ),
                      RadioListTile<String>(
                        title: const Text('选择性同步'),
                        subtitle: const Text('仅同步指定目录'),
                        value: 'selective',
                      ),
                      if (Platform.isAndroid)
                        RadioListTile<String>(
                          title: const Text('相册同步'),
                          subtitle: const Text('自动备份手机照片到云端'),
                          value: 'album',
                        ),
                    ],
                  ),
                ),
              ],
            ),
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
            _buildSection(
              title: '性能',
              children: [
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

  Future<void> _editRemoteRoot() async {
    final controller = TextEditingController(text: _remoteRoot);
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('远程同步目录'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            hintText: 'cloudreve://my',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('确定'),
          ),
        ],
      ),
    );
    if (result != null && result.isNotEmpty) {
      setState(() => _remoteRoot = result);
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

  /// 配置变更后，如果引擎在运行中，实时推送到 Rust
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
}
