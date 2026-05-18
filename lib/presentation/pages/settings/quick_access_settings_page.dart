import 'package:cloudreve4_flutter/core/constants/quick_access_defaults.dart';
import 'package:cloudreve4_flutter/presentation/providers/quick_access_provider.dart';
import 'package:cloudreve4_flutter/presentation/widgets/toast_helper.dart';
import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provider/provider.dart';

class QuickAccessSettingsPage extends StatelessWidget {
  const QuickAccessSettingsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.watch<QuickAccessProvider>();

    return Scaffold(
      appBar: AppBar(title: const Text('快捷入口')),
      body: ListView(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(
              '自定义概览页中显示的快捷目录入口。默认入口不可删除，但可编辑路径和调整顺序。',
              style: theme.textTheme.bodyMedium?.copyWith(color: theme.hintColor),
            ),
          ),
          if (provider.isLoaded)
            ...List.generate(provider.items.length, (index) {
              final item = provider.items[index];
              return Card(
                margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
                child: ListTile(
                  leading: Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: item.color.withValues(alpha: 0.2),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Icon(item.icon, size: 20, color: item.color.darken(0.3)),
                  ),
                  title: Row(
                    children: [
                      Text(item.label),
                      if (item.isDefault) ...[
                        const SizedBox(width: 6),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.primary.withValues(alpha: 0.1),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text('默认', style: TextStyle(fontSize: 10, color: theme.colorScheme.primary, fontWeight: FontWeight.w600)),
                        ),
                      ],
                    ],
                  ),
                  subtitle: Text(item.path, style: TextStyle(color: theme.hintColor, fontSize: 12)),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: Icon(LucideIcons.chevronUp, size: 18),
                        onPressed: index > 0 ? () => provider.moveItem(index, index - 1) : null,
                        tooltip: '上移',
                        visualDensity: VisualDensity.compact,
                      ),
                      IconButton(
                        icon: Icon(LucideIcons.chevronDown, size: 18),
                        onPressed: index < provider.items.length - 1 ? () => provider.moveItem(index, index + 1) : null,
                        tooltip: '下移',
                        visualDensity: VisualDensity.compact,
                      ),
                      IconButton(
                        icon: Icon(LucideIcons.pencil, size: 16),
                        onPressed: () => _editItem(context, provider, index),
                        tooltip: '编辑',
                        visualDensity: VisualDensity.compact,
                      ),
                      if (!item.isDefault)
                        IconButton(
                          icon: Icon(LucideIcons.trash2, size: 16, color: theme.colorScheme.error),
                          onPressed: () {
                            provider.deleteItem(index);
                            ToastHelper.success('快捷入口已删除');
                          },
                          tooltip: '删除',
                          visualDensity: VisualDensity.compact,
                        ),
                    ],
                  ),
                ),
              );
            }),
          const SizedBox(height: 16),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Expanded(
                  child: FilledButton.icon(
                    icon: const Icon(LucideIcons.plus, size: 18),
                    label: const Text('新增快捷入口'),
                    onPressed: () => _addItem(context, provider),
                  ),
                ),
                const SizedBox(width: 12),
                OutlinedButton.icon(
                  icon: const Icon(LucideIcons.rotateCcw, size: 16),
                  label: const Text('恢复默认'),
                  onPressed: () {
                    provider.resetToDefaults();
                    ToastHelper.success('已恢复默认设置');
                  },
                ),
              ],
            ),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }

  Future<void> _editItem(BuildContext context, QuickAccessProvider provider, int index) async {
    final item = provider.items[index];
    final labelController = TextEditingController(text: item.label);
    final pathController = TextEditingController(text: item.path);

    final result = await showDialog<_EditResult>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('编辑快捷入口'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: labelController,
              decoration: const InputDecoration(labelText: '名称', hintText: '例如: 图片'),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: pathController,
              decoration: const InputDecoration(labelText: '目录路径', hintText: '例如: /Images'),
            ),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(ctx).pop(), child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(_EditResult(labelController.text, pathController.text)),
            child: const Text('确定'),
          ),
        ],
      ),
    );

    if (result != null) {
      provider.updateItem(
        index,
        item.copyWith(
          label: result.label.isNotEmpty ? result.label : item.label,
          path: result.path.isNotEmpty ? result.path : item.path,
        ),
      );
      ToastHelper.success('快捷入口已更新');
    }
  }

  Future<void> _addItem(BuildContext context, QuickAccessProvider provider) async {
    final labelController = TextEditingController();
    final pathController = TextEditingController();
    IconData selectedIcon = LucideIcons.folder;
    Color selectedColor = QuickAccessConfig.colorPool[0];

    final result = await showDialog<_AddResult>(
      context: context,
      builder: (ctx) {
        return StatefulBuilder(
          builder: (ctx, setDialogState) {
            return AlertDialog(
              title: const Text('新增快捷入口'),
              content: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    TextField(
                      controller: labelController,
                      decoration: const InputDecoration(labelText: '名称', hintText: '例如: 图片'),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: pathController,
                      decoration: const InputDecoration(labelText: '目录路径', hintText: '例如: /Images'),
                    ),
                    const SizedBox(height: 16),
                    Text('图标', style: Theme.of(ctx).textTheme.bodySmall?.copyWith(color: Theme.of(ctx).hintColor)),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: QuickAccessConfig.iconPool.map((icon) {
                        final isSelected = icon.codePoint == selectedIcon.codePoint;
                        return GestureDetector(
                          onTap: () => setDialogState(() => selectedIcon = icon),
                          child: Container(
                            width: 40,
                            height: 40,
                            decoration: BoxDecoration(
                              color: isSelected ? Theme.of(ctx).colorScheme.primary.withValues(alpha: 0.15) : null,
                              borderRadius: BorderRadius.circular(10),
                              border: isSelected
                                  ? Border.all(color: Theme.of(ctx).colorScheme.primary, width: 2)
                                  : Border.all(color: Theme.of(ctx).dividerColor),
                            ),
                            child: Icon(icon, size: 20, color: isSelected ? Theme.of(ctx).colorScheme.primary : Theme.of(ctx).hintColor),
                          ),
                        );
                      }).toList(),
                    ),
                    const SizedBox(height: 16),
                    Text('颜色', style: Theme.of(ctx).textTheme.bodySmall?.copyWith(color: Theme.of(ctx).hintColor)),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: QuickAccessConfig.colorPool.map((color) {
                        final isSelected = color.toARGB32() == selectedColor.toARGB32();
                        return GestureDetector(
                          onTap: () => setDialogState(() => selectedColor = color),
                          child: Container(
                            width: 36,
                            height: 36,
                            decoration: BoxDecoration(
                              color: color.withValues(alpha: 0.6),
                              borderRadius: BorderRadius.circular(10),
                              border: isSelected
                                  ? Border.all(color: color.darken(0.2), width: 3)
                                  : null,
                            ),
                            child: isSelected
                                ? Icon(LucideIcons.check, size: 18, color: color.darken(0.3))
                                : null,
                          ),
                        );
                      }).toList(),
                    ),
                  ],
                ),
              ),
              actions: [
                TextButton(onPressed: () => Navigator.of(ctx).pop(), child: const Text('取消')),
                FilledButton(
                  onPressed: () => Navigator.of(ctx).pop(_AddResult(
                    labelController.text,
                    pathController.text,
                    selectedIcon,
                    selectedColor,
                  )),
                  child: const Text('添加'),
                ),
              ],
            );
          },
        );
      },
    );

    if (result != null && result.label.isNotEmpty && result.path.isNotEmpty) {
      provider.addItem(QuickAccessConfig(
        id: 'custom_${DateTime.now().millisecondsSinceEpoch}',
        label: result.label,
        icon: result.icon,
        path: result.path,
        color: result.color,
      ));
      ToastHelper.success('快捷入口已添加');
    }
  }
}

class _EditResult {
  final String label;
  final String path;
  _EditResult(this.label, this.path);
}

class _AddResult {
  final String label;
  final String path;
  final IconData icon;
  final Color color;
  _AddResult(this.label, this.path, this.icon, this.color);
}
