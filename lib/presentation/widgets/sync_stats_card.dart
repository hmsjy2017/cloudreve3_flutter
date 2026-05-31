import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

/// 同步统计卡片 - 左侧饼图 + 右侧明细列表
class SyncStatsCard extends StatelessWidget {
  final int uploaded;
  final int downloaded;
  final int renamed;
  final int moved;
  final int conflicts;
  final int failed;

  const SyncStatsCard({
    super.key,
    required this.uploaded,
    required this.downloaded,
    required this.renamed,
    required this.moved,
    required this.conflicts,
    required this.failed,
  });

  static const _colorUploaded = Color(0xFF8E5D67);
  static const _colorDownloaded = Color(0xFF8BA7DA);
  static const _colorRenamed = Color(0xFFD4AF37);
  static const _colorMoved = Color(0xFF67B5B1);
  static const _colorConflicts = Color(0xFFE69A6A);
  static const _colorFailed = Color(0xFFD9534F);

  int get _total => uploaded + downloaded + renamed + moved + conflicts + failed;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;
    final bgColor = isDark ? theme.colorScheme.surfaceContainerHigh : Colors.white;

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(32),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.04),
            blurRadius: 20,
            offset: const Offset(0, 10),
          ),
        ],
      ),
      child: Row(
        children: [
          // 左侧饼图
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
                      centerSpaceRadius: 36,
                      sections: _buildSections(),
                    ),
                  ),
                ),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.sync_rounded, color: theme.colorScheme.primary, size: 20),
                    const SizedBox(height: 2),
                    Text(
                      '$_total',
                      style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(width: 24),
          // 右侧明细
          Expanded(
            flex: 6,
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                _buildDetailRow('上传', uploaded, _colorUploaded, theme),
                _buildDetailRow('下载', downloaded, _colorDownloaded, theme),
                _buildDetailRow('重命名', renamed, _colorRenamed, theme),
                _buildDetailRow('移动', moved, _colorMoved, theme),
                _buildDetailRow('冲突', conflicts, _colorConflicts, theme),
                _buildDetailRow('失败', failed, _colorFailed, theme),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildDetailRow(String label, int count, Color color, ThemeData theme) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 10),
          Text(label, style: TextStyle(fontSize: 13, color: theme.hintColor)),
          const Spacer(),
          Text(
            '$count',
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
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
