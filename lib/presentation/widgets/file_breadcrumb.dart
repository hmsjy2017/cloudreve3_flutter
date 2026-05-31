import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';

/// 面包屑导航组件（桌面端底部）
class FileBreadcrumb extends StatefulWidget {
  final String currentPath;
  final void Function(String path) onPathTap;

  const FileBreadcrumb({
    super.key,
    required this.currentPath,
    required this.onPathTap,
  });

  @override
  State<FileBreadcrumb> createState() => _FileBreadcrumbState();
}

class _FileBreadcrumbState extends State<FileBreadcrumb> {
  final _controller = ScrollController();

  @override
  void didUpdateWidget(covariant FileBreadcrumb oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentPath != widget.currentPath) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_controller.hasClients) {
          _controller.animateTo(
            _controller.position.maxScrollExtent,
            duration: const Duration(milliseconds: 200),
            curve: Curves.easeOut,
          );
        }
      });
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final pathParts = widget.currentPath.split('/');
    pathParts.removeWhere((part) => part.isEmpty);
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      decoration: BoxDecoration(
        color: theme.scaffoldBackgroundColor,
        border: Border(
          top: BorderSide(color: theme.dividerColor.withValues(alpha: 0.5), width: 1),
        ),
      ),
      child: SingleChildScrollView(
        controller: _controller,
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            _buildBreadcrumbItem(
              context,
              name: '首页',
              path: '/',
              icon: LucideIcons.home,
              primaryColor: colorScheme.primary,
              onTap: () => widget.onPathTap('/'),
            ),
            for (int i = 0; i < pathParts.length; i++) ...[
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: Icon(LucideIcons.chevronRight, size: 16, color: theme.hintColor.withValues(alpha: 0.5)),
              ),
              _buildBreadcrumbItem(
                context,
                name: _decodePathSegment(pathParts[i]),
                path: '/${pathParts.sublist(0, i + 1).join('/')}',
                icon: null,
                primaryColor: colorScheme.primary,
                onTap: () => widget.onPathTap('/${pathParts.sublist(0, i + 1).join('/')}'),
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _decodePathSegment(String segment) {
    var decoded = segment;
    for (var i = 0; i < 5; i++) {
      try {
        final next = Uri.decodeComponent(decoded);
        if (next == decoded) break;
        decoded = next;
      } catch (_) {
        break;
      }
    }
    return decoded;
  }

  Widget _buildBreadcrumbItem(
    BuildContext context, {
    required String name,
    required String path,
    required IconData? icon,
    required Color primaryColor,
    required VoidCallback onTap,
  }) {
    return Container(
      height: 36,
      decoration: BoxDecoration(
        color: primaryColor.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(18),
      ),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(18),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          child: Row(
            children: [
              if (icon != null)
                Icon(icon, size: 16, color: primaryColor),
              if (icon != null) const SizedBox(width: 5),
              Text(
                name,
                style: TextStyle(
                  color: primaryColor,
                  fontWeight: FontWeight.w600,
                  fontSize: 13,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
