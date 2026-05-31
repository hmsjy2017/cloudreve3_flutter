/// 排序字段
enum SortField {
  name('名称', 'name'),
  size('大小', 'size'),
  updatedAt('修改时间', 'updated_at'),
  createdAt('创建时间', 'created_at');

  final String label;
  final String apiKey;
  const SortField(this.label, this.apiKey);
}

/// 排序方向
enum SortDirection {
  asc('升序', 'asc'),
  desc('降序', 'desc');

  final String label;
  final String apiKey;
  const SortDirection(this.label, this.apiKey);
}

/// 排序选项
class SortOption {
  final SortField field;
  final SortDirection direction;

  const SortOption(this.field, this.direction);

  static const default_ = SortOption(SortField.name, SortDirection.asc);

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SortOption && field == other.field && direction == other.direction;

  @override
  int get hashCode => Object.hash(field, direction);

  /// 持久化字符串，格式: "name_asc"
  String toKey() => '${field.apiKey}_${direction.apiKey}';

  /// 从持久化字符串恢复
  static SortOption fromKey(String? key) {
    if (key == null) return default_;
    final parts = key.split('_');
    if (parts.length != 2) return default_;
    final field = SortField.values.where((f) => f.apiKey == parts[0]).firstOrNull;
    final dir = SortDirection.values.where((d) => d.apiKey == parts[1]).firstOrNull;
    if (field == null || dir == null) return default_;
    return SortOption(field, dir);
  }

  /// 生成菜单项显示文本
  String get menuLabel {
    final dirLabel = switch (field) {
      SortField.name => direction == SortDirection.asc ? 'A→Z' : 'Z→A',
      SortField.size => direction == SortDirection.asc ? '小→大' : '大→小',
      SortField.updatedAt => direction == SortDirection.asc ? '旧→新' : '新→旧',
      SortField.createdAt => direction == SortDirection.asc ? '旧→新' : '新→旧',
    };
    return '${field.label} $dirLabel';
  }
}
