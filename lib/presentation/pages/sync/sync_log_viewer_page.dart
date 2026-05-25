import 'dart:io';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import '../../widgets/toast_helper.dart';

/// 同步引擎日志预览页面
class SyncLogViewerPage extends StatefulWidget {
  const SyncLogViewerPage({super.key});

  @override
  State<SyncLogViewerPage> createState() => _SyncLogViewerPageState();
}

class _SyncLogViewerPageState extends State<SyncLogViewerPage> {
  String _logContent = '';
  bool _isLoading = true;
  final ScrollController _scrollController = ScrollController();

  @override
  void initState() {
    super.initState();
    _loadLog();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  Future<String> _getSyncLogPath() async {
    final appDir = await getApplicationSupportDirectory();
    return '${appDir.path}${Platform.pathSeparator}sync_core${Platform.pathSeparator}logs${Platform.pathSeparator}sync_log.txt';
  }

  Future<void> _loadLog() async {
    setState(() => _isLoading = true);
    try {
      final path = await _getSyncLogPath();
      final file = File(path);
      if (!await file.exists()) {
        if (mounted) {
          setState(() {
            _logContent = '';
            _isLoading = false;
          });
        }
        return;
      }

      final lines = await file.readAsLines();
      const maxLines = 1000;
      String content;
      if (lines.length <= maxLines) {
        content = lines.join('\n');
      } else {
        content = '... (仅显示最近 $maxLines 行)\n\n'
            '${lines.sublist(lines.length - maxLines).join('\n')}';
      }

      if (mounted) {
        setState(() {
          _logContent = content;
          _isLoading = false;
        });
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (_scrollController.hasClients) {
            _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
          }
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() => _isLoading = false);
        ToastHelper.error('读取同步日志失败：$e');
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;

    return Scaffold(
      appBar: AppBar(
        title: const Text('同步日志'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadLog,
            tooltip: '刷新',
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : _logContent.isEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.description_outlined,
                          size: 48, color: theme.hintColor.withValues(alpha: 0.4)),
                      const SizedBox(height: 16),
                      Text('暂无同步日志', style: TextStyle(color: theme.hintColor)),
                    ],
                  ),
                )
              : Container(
                  color: isDark ? const Color(0xFF1E1E1E) : const Color(0xFFF5F5F5),
                  child: Scrollbar(
                    controller: _scrollController,
                    child: SingleChildScrollView(
                      controller: _scrollController,
                      padding: const EdgeInsets.all(12),
                      child: SelectableText(
                        _logContent,
                        style: TextStyle(
                          fontFamily: 'SourceCodePro',
                          fontSize: 13,
                          height: 1.5,
                          color: isDark ? Colors.grey[300] : Colors.grey[900],
                        ),
                      ),
                    ),
                  ),
                ),
    );
  }
}
