import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

/// 同步统计卡片
/// 宽屏（>=400）：左饼图 + 右双列明细（左5右4）
/// 窄屏（<400）：上饼图+5指标，下4指标
class SyncStatsCard extends StatelessWidget {
  final int uploaded;
  final int downloaded;
  final int renamed;
  final int moved;
  final int conflicts;
  final int failed;
  final int deletedLocal;
  final int deletedRemote;
  final int skipped;

  const SyncStatsCard({
    super.key,
    required this.uploaded,
    required this.downloaded,
    required this.renamed,
    required this.moved,
    required this.conflicts,
    required this.failed,
    required this.deletedLocal,
    required this.deletedRemote,
    required this.skipped,
  });

  static const _colorUploaded = Color(0xFF8E5D67);
  static const _colorDownloaded = Color(0xFF8BA7DA);
  static const _colorRenamed = Color(0xFFD4AF37);
  static const _colorMoved = Color(0xFF67B5B1);
  static const _colorConflicts = Color(0xFFE69A6A);
  static const _colorFailed = Color(0xFFD9534F);
  static const _colorDeletedLocal = Color(0xFFE57373);
  static const _colorDeletedRemote = Color(0xFFEF9A9A);
  static const _colorSkipped = Color(0xFFB0BEC5);

  static const _narrowThreshold = 400.0;

  int get _total =>
      uploaded +
      downloaded +
      renamed +
      moved +
      conflicts +
      failed +
      deletedLocal +
      deletedRemote +
      skipped;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final decoration = BoxDecoration(
      gradient: LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [
          theme.colorScheme.primaryContainer.withValues(alpha: 0.3),
          theme.colorScheme.tertiaryContainer.withValues(alpha: 0.3),
        ],
      ),
      borderRadius: BorderRadius.circular(24),
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final isWide = constraints.maxWidth >= _narrowThreshold;

        if (isWide) {
          return Container(
            width: double.infinity,
            padding: const EdgeInsets.all(24),
            decoration: decoration,
            child: Row(
              children: [
                // 饼图
                Expanded(
                  flex: 4,
                  child: Stack(
                    alignment: Alignment.center,
                    children: [
                      SizedBox(
                        height: 160,
                        child: PieChart(
                          PieChartData(
                            sectionsSpace: 4,
                            centerSpaceRadius: 42,
                            sections: _buildSections(),
                          ),
                        ),
                      ),
                      Text(
                        '$_total',
                        style: theme.textTheme.bodyLarge?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 16),
                // 双列明细
                Expanded(
                  flex: 6,
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: [
                      // 左列 5 项
                      Expanded(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildRow('上传', uploaded, _colorUploaded, theme),
                            _buildRow('下载', downloaded, _colorDownloaded, theme),
                            _buildRow('移动', moved, _colorMoved, theme),
                            _buildRow('冲突', conflicts, _colorConflicts, theme),
                            _buildRow('失败', failed, _colorFailed, theme),
                          ],
                        ),
                      ),
                      const SizedBox(width: 12),
                      // 右列 4 项
                      Expanded(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildRow('重命名', renamed, _colorRenamed, theme),
                            _buildRow('删本地', deletedLocal, _colorDeletedLocal, theme),
                            _buildRow('删远程', deletedRemote, _colorDeletedRemote, theme),
                            _buildRow('跳过', skipped, _colorSkipped, theme),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          );
        }

        // 窄屏：上下布局
        return Container(
          width: double.infinity,
          padding: const EdgeInsets.fromLTRB(20, 20, 20, 24),
          decoration: decoration,
          child: Column(
            children: [
              // 上半区：饼图 + 5 个主指标
              Row(
                children: [
                  SizedBox(
                    width: 120,
                    height: 120,
                    child: Stack(
                      alignment: Alignment.center,
                      children: [
                        PieChart(
                          PieChartData(
                            sectionsSpace: 3,
                            centerSpaceRadius: 32,
                            sections: _buildSections(),
                          ),
                        ),
                        Text(
                          '$_total',
                          style: theme.textTheme.bodyLarge?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 20),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        _buildRow('上传', uploaded, _colorUploaded, theme),
                        _buildRow('下载', downloaded, _colorDownloaded, theme),
                        _buildRow('移动', moved, _colorMoved, theme),
                        _buildRow('冲突', conflicts, _colorConflicts, theme),
                        _buildRow('失败', failed, _colorFailed, theme),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              // 下半区：4 个副指标
              Row(
                children: [
                  _buildSubItem('重命名', renamed, _colorRenamed, theme),
                  const SizedBox(width: 16),
                  _buildSubItem('删本地', deletedLocal, _colorDeletedLocal, theme),
                  const SizedBox(width: 16),
                  _buildSubItem('删远程', deletedRemote, _colorDeletedRemote, theme),
                  const SizedBox(width: 16),
                  _buildSubItem('跳过', skipped, _colorSkipped, theme),
                ],
              ),
            ],
          ),
        );
      },
    );
  }

  /// 指标行：圆点 + 固定宽标签 + 数字，保证数字对齐
  Widget _buildRow(String label, int count, Color color, ThemeData theme) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3.5),
      child: Row(
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 6),
          SizedBox(
            width: 40,
            child: Text(label, style: TextStyle(fontSize: 12, color: theme.hintColor)),
          ),
          Text(
            '$count',
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
          ),
        ],
      ),
    );
  }

  /// 窄屏副指标：圆点 + 标签 + 数字（紧凑，Expanded 均分）
  Widget _buildSubItem(String label, int count, Color color, ThemeData theme) {
    return Expanded(
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 4),
          Flexible(
            child: Text(label,
                style: TextStyle(fontSize: 11, color: theme.hintColor),
                overflow: TextOverflow.ellipsis),
          ),
          const SizedBox(width: 3),
          Text(
            '$count',
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 12),
          ),
        ],
      ),
    );
  }

  List<PieChartSectionData> _buildSections() {
    if (_total == 0) {
      return [
        PieChartSectionData(
          value: 1,
          color: const Color(0xFFE0E0E0),
          radius: 14,
          showTitle: false,
        ),
      ];
    }

    final entries = [
      (uploaded, _colorUploaded, 18.0),
      (downloaded, _colorDownloaded, 16.0),
      (renamed, _colorRenamed, 14.0),
      (moved, _colorMoved, 12.0),
      (conflicts, _colorConflicts, 10.0),
      (failed, _colorFailed, 8.0),
      (deletedLocal, _colorDeletedLocal, 8.0),
      (deletedRemote, _colorDeletedRemote, 6.0),
      (skipped, _colorSkipped, 6.0),
    ];

    return entries
        .where((e) => e.$1 > 0)
        .map((e) => PieChartSectionData(
              value: e.$1.toDouble(),
              color: e.$2,
              radius: e.$3,
              showTitle: false,
            ))
        .toList();
  }
}
