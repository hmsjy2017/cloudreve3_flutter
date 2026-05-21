import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:lucide_icons/lucide_icons.dart';

import '../../../data/models/sync_task_model.dart';
import '../../providers/sync_provider.dart';
import '../../widgets/desktop_constrained.dart';
import '../../widgets/toast_helper.dart';
import 'sync_settings_page.dart';

/// 同步 Tab 页 - 展示实时同步状态、活跃任务和已完成任务
class SyncPage extends StatefulWidget {
  const SyncPage({super.key});

  @override
  State<SyncPage> createState() => _SyncPageState();
}

class _SyncPageState extends State<SyncPage> {
  // 展开的任务 ID
  final Set<String> _expandedTasks = {};
  // 正在加载详情的任务 ID
  final Set<String> _loadingDetails = {};

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<SyncProvider>().loadRecentTasks();
    });
  }

  @override
  Widget build(BuildContext context) {
    final sync = context.watch<SyncProvider>();
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('文件同步'),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings_outlined),
            onPressed: () => _navigateToSettings(),
            tooltip: '同步设置',
          ),
        ],
      ),
      body: DesktopConstrained(
        child: RefreshIndicator(
          onRefresh: () async {
            final sync = context.read<SyncProvider>();
            sync.invalidateAllTaskDetails();
            await sync.loadRecentTasks();
          },
          child: ListView(
            padding: const EdgeInsets.symmetric(vertical: 8),
            children: [
              _buildStatusCard(sync, theme),
              const SizedBox(height: 8),
              _buildActiveTasksSection(sync, theme),
              const SizedBox(height: 8),
              _buildCompletedTasksSection(sync, theme),
              const SizedBox(height: 32),
            ],
          ),
        ),
      ),
    );
  }

  void _navigateToSettings() {
    Navigator.of(context).push(
      MaterialPageRoute(builder: (_) => const SyncSettingsPage()),
    );
  }

  Widget _buildStatusCard(SyncProvider sync, ThemeData theme) {
    final isActive = sync.isActive;
    final isPaused = sync.isPaused;
    final hasError = sync.hasError;

    Color statusColor;
    IconData statusIcon;
    String statusText;

    if (isActive) {
      statusColor = theme.colorScheme.primary;
      statusIcon = Icons.sync;
      statusText = _syncModeLabel(sync);
    } else if (isPaused) {
      statusColor = Colors.orange;
      statusIcon = Icons.pause_circle_outline;
      statusText = '已暂停';
    } else if (hasError) {
      statusColor = theme.colorScheme.error;
      statusIcon = Icons.error_outline;
      statusText = '同步错误';
    } else if (sync.state == SyncState.stopped) {
      statusColor = theme.disabledColor;
      statusIcon = Icons.stop_circle_outlined;
      statusText = '已停止';
    } else {
      statusColor = theme.disabledColor;
      statusIcon = Icons.cloud_off;
      statusText = '未启动';
    }

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(statusIcon, color: statusColor, size: 28),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        statusText,
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      if (sync.errorMessage != null)
                        Text(
                          sync.errorMessage!,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.error,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                    ],
                  ),
                ),
                if (sync.activeWorkerCount > 0)
                  Badge(
                    label: Text('${sync.activeWorkerCount}'),
                    child: Icon(LucideIcons.loader, color: statusColor, size: 24),
                  ),
              ],
            ),
            if (isActive || isPaused) ...[
              const SizedBox(height: 12),
              ClipRRect(
                borderRadius: BorderRadius.circular(4),
                child: LinearProgressIndicator(
                  value: sync.totalFiles > 0 ? sync.progress : null,
                  minHeight: 6,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                sync.totalFiles > 0
                    ? '${sync.syncedFiles} / ${sync.totalFiles} 文件'
                    : sync.state == SyncState.continuous
                        ? '持续同步中'
                        : '正在同步...',
                style: theme.textTheme.bodySmall,
              ),
            ],
            if (sync.lastSummary != null) ...[
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 4,
                children: [
                  _summaryChip(theme, '上传', sync.lastSummary!.uploaded),
                  _summaryChip(theme, '下载', sync.lastSummary!.downloaded),
                  _summaryChip(theme, '冲突', sync.lastSummary!.conflicts),
                  _summaryChip(theme, '失败', sync.lastSummary!.failed),
                  _summaryChip(theme, '重命名', sync.lastSummary!.renamed),
                  _summaryChip(theme, '移动', sync.lastSummary!.moved),
                ],
              ),
            ],
            const SizedBox(height: 12),
            Row(
              children: [
                if (!sync.isActive && !sync.isPaused)
                  Expanded(
                    child: FilledButton.icon(
                      onPressed: () => _navigateToSettings(),
                      icon: const Icon(Icons.play_arrow, size: 18),
                      label: const Text('开始同步'),
                    ),
                  ),
                if (sync.isActive)
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: () => sync.pause(),
                      icon: const Icon(Icons.pause, size: 18),
                      label: const Text('暂停'),
                    ),
                  ),
                if (sync.isPaused)
                  Expanded(
                    child: FilledButton.icon(
                      onPressed: () => sync.resume(),
                      icon: const Icon(Icons.play_arrow, size: 18),
                      label: const Text('恢复'),
                    ),
                  ),
                if (sync.isActive || sync.isPaused) ...[
                  const SizedBox(width: 8),
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: () => _stopSync(sync),
                      icon: const Icon(Icons.stop, size: 18),
                      label: const Text('停止'),
                      style: OutlinedButton.styleFrom(
                        foregroundColor: theme.colorScheme.error,
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _summaryChip(ThemeData theme, String label, int value) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('$label:', style: theme.textTheme.bodySmall?.copyWith(color: theme.hintColor)),
        Text(' $value', style: theme.textTheme.bodySmall?.copyWith(fontWeight: FontWeight.w600)),
      ],
    );
  }

  String _syncModeLabel(SyncProvider sync) {
    final mode = sync.persistedConfig?.syncMode ?? 'full';
    return switch (mode) {
      'full' => '全量同步中',
      'upload_only' => '仅上传中',
      'download_only' => '仅下载中',
      'album' => '相册同步中',
      _ => '同步中',
    };
  }

  Widget _buildActiveTasksSection(SyncProvider sync, ThemeData theme) {
    final tasks = sync.activeTasks;

    return _buildTaskSection(
      theme: theme,
      title: '正在同步',
      icon: LucideIcons.loader,
      tasks: tasks,
      emptyText: '暂无正在进行的同步任务',
    );
  }

  Widget _buildCompletedTasksSection(SyncProvider sync, ThemeData theme) {
    final tasks = sync.recentTasks
        .where((t) =>
            (t.status == 'completed' || t.status == 'failed' || t.status == 'cancelled') &&
            t.totalCount > 0)
        .toList();

    return _buildTaskSection(
      theme: theme,
      title: '已完成',
      icon: LucideIcons.checkCircle2,
      tasks: tasks,
      emptyText: '暂无已完成的同步任务',
    );
  }

  Widget _buildTaskSection({
    required ThemeData theme,
    required String title,
    required IconData icon,
    required List<SyncTaskModel> tasks,
    required String emptyText,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Icon(icon, size: 18, color: theme.colorScheme.primary),
              const SizedBox(width: 8),
              Text(
                title,
                style: theme.textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: theme.colorScheme.primary,
                ),
              ),
            ],
          ),
        ),
        if (tasks.isEmpty)
          Card(
            margin: const EdgeInsets.symmetric(horizontal: 16),
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Center(
                child: Text(emptyText, style: TextStyle(color: theme.hintColor)),
              ),
            ),
          )
        else
          ...tasks.map((task) => _buildTaskCard(task, theme)),
      ],
    );
  }

  Widget _buildTaskCard(SyncTaskModel task, ThemeData theme) {
    final isExpanded = _expandedTasks.contains(task.id);

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 2),
      child: Column(
        children: [
          InkWell(
            onTap: () => _toggleTaskExpand(task.id),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              child: Row(
                children: [
                  GestureDetector(
                    onTap: () {
                      Clipboard.setData(ClipboardData(text: task.id));
                      ToastHelper.success('已复制任务 ID');
                    },
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SizedBox(
                          width: 72,
                          child: Text(
                            task.shortId,
                            style: theme.textTheme.bodySmall?.copyWith(
                              fontFamily: 'monospace',
                              color: theme.hintColor,
                            ),
                          ),
                        ),
                        Icon(Icons.copy, size: 14, color: theme.hintColor),
                      ],
                    ),
                  ),
                  Expanded(
                    child: Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                          decoration: BoxDecoration(
                            color: _statusColor(task.status, theme).withValues(alpha: 0.1),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            task.statusLabel,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: _statusColor(task.status, theme),
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                        const SizedBox(width: 12),
                        Text(
                          '${task.completedCount}/${task.totalCount}',
                          style: theme.textTheme.bodySmall,
                        ),
                        if (task.failedCount > 0) ...[
                          const SizedBox(width: 8),
                          Text(
                            '失败${task.failedCount}',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.error,
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                  Icon(
                    isExpanded ? Icons.expand_less : Icons.expand_more,
                    size: 20,
                    color: theme.hintColor,
                  ),
                ],
              ),
            ),
          ),
          if (isExpanded)
            _buildTaskDetailList(task, theme),
        ],
      ),
    );
  }

  Widget _buildTaskDetailList(SyncTaskModel task, ThemeData theme) {
    final sync = context.watch<SyncProvider>();
    final items = sync.getCachedTaskDetail(task.id);

    if (_loadingDetails.contains(task.id)) {
      return const Padding(
        padding: EdgeInsets.all(16),
        child: Center(child: SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))),
      );
    }

    if (items == null || items.isEmpty) {
      return Padding(
        padding: const EdgeInsets.all(16),
        child: Text('暂无任务项', style: TextStyle(color: theme.hintColor)),
      );
    }

    return Column(
      children: [
        const Divider(height: 1, indent: 16, endIndent: 16),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
          child: Row(
            children: [
              SizedBox(width: 70, child: Text('操作', style: theme.textTheme.labelSmall?.copyWith(color: theme.hintColor))),
              Expanded(child: Text('文件名', style: theme.textTheme.labelSmall?.copyWith(color: theme.hintColor))),
              SizedBox(width: 55, child: Text('状态', style: theme.textTheme.labelSmall?.copyWith(color: theme.hintColor))),
              SizedBox(width: 110, child: Text('时间', style: theme.textTheme.labelSmall?.copyWith(color: theme.hintColor))),
            ],
          ),
        ),
        ...items.map((item) => _buildTaskItemRow(item, theme)),
        const SizedBox(height: 4),
      ],
    );
  }

  Widget _buildTaskItemRow(SyncTaskItemModel item, ThemeData theme) {
    final timeStr = _formatTime(item.updatedAt);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 3),
      child: Row(
        children: [
          SizedBox(
            width: 70,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
              decoration: BoxDecoration(
                color: _actionColor(item.actionType, theme).withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(3),
              ),
              child: Text(
                item.actionLabel,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontSize: 11,
                  color: _actionColor(item.actionType, theme),
                ),
              ),
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              item.filename,
              style: theme.textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
              maxLines: 1,
            ),
          ),
          SizedBox(
            width: 55,
            child: Text(
              item.statusLabel,
              style: theme.textTheme.bodySmall?.copyWith(
                color: _itemStatusColor(item.status, theme),
              ),
            ),
          ),
          SizedBox(
            width: 110,
            child: Text(
              timeStr,
              style: theme.textTheme.bodySmall?.copyWith(
                fontFamily: 'monospace',
                fontSize: 11,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Color _statusColor(String status, ThemeData theme) {
    return switch (status) {
      'running' => theme.colorScheme.primary,
      'completed' => Colors.green,
      'failed' => theme.colorScheme.error,
      'cancelled' => theme.hintColor,
      _ => theme.hintColor,
    };
  }

  Color _actionColor(String actionType, ThemeData theme) {
    return switch (actionType) {
      'upload' => Colors.blue,
      'download' => Colors.green,
      'delete_local' || 'delete_remote' => theme.colorScheme.error,
      'rename' || 'move' => Colors.orange,
      'conflict_resolve' => Colors.purple,
      _ => theme.colorScheme.primary,
    };
  }

  Color _itemStatusColor(String status, ThemeData theme) {
    return switch (status) {
      'completed' => Colors.green,
      'failed' => theme.colorScheme.error,
      'running' => theme.colorScheme.primary,
      _ => theme.hintColor,
    };
  }

  String _formatTime(String isoTime) {
    try {
      final dt = DateTime.parse(isoTime).toLocal();
      return '${dt.year}/${dt.month.toString().padLeft(2, '0')}/${dt.day.toString().padLeft(2, '0')} '
          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
    } catch (_) {
      return isoTime;
    }
  }

  void _toggleTaskExpand(String taskId) {
    if (_expandedTasks.contains(taskId)) {
      setState(() => _expandedTasks.remove(taskId));
      return;
    }

    setState(() => _expandedTasks.add(taskId));

    final sync = context.read<SyncProvider>();
    if (sync.getCachedTaskDetail(taskId) == null && !_loadingDetails.contains(taskId)) {
      setState(() => _loadingDetails.add(taskId));
      sync.getTaskDetail(taskId).whenComplete(() {
        if (mounted) {
          setState(() => _loadingDetails.remove(taskId));
        }
      });
    }
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
