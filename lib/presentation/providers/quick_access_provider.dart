import 'package:flutter/foundation.dart';

import '../../core/constants/quick_access_defaults.dart';
import '../../services/storage_service.dart';

class QuickAccessProvider extends ChangeNotifier {
  List<QuickAccessConfig> _items = List.from(QuickAccessConfig.defaults);
  bool _isLoaded = false;

  List<QuickAccessConfig> get items => _items;
  bool get isLoaded => _isLoaded;

  Future<void> load() async {
    if (_isLoaded) return;

    final saved = await StorageService.instance
        .getString(QuickAccessConfig.storageKey);
    if (saved != null && saved.isNotEmpty) {
      try {
        _items = QuickAccessConfig.parseSaved(saved);
        _isLoaded = true;
        notifyListeners();
        return;
      } catch (_) {}
    }

    // 迁移 v1
    final v1 = await StorageService.instance
        .getString('quick_access_shortcuts');
    if (v1 != null && v1.isNotEmpty) {
      _items = QuickAccessConfig.migrateV1(v1);
      _isLoaded = true;
      await _save();
      notifyListeners();
      return;
    }

    _items = List.from(QuickAccessConfig.defaults);
    _isLoaded = true;
    notifyListeners();
  }

  Future<void> addItem(QuickAccessConfig item) async {
    _items = [..._items, item];
    await _save();
    notifyListeners();
  }

  Future<void> updateItem(int index, QuickAccessConfig item) async {
    _items = [..._items]..[index] = item;
    await _save();
    notifyListeners();
  }

  Future<void> moveItem(int from, int to) async {
    if (from < 0 || from >= _items.length || to < 0 || to >= _items.length || from == to) return;
    final list = [..._items];
    final item = list.removeAt(from);
    list.insert(to, item);
    _items = list;
    await _save();
    notifyListeners();
  }

  Future<void> deleteItem(int index) async {
    _items = [..._items]..removeAt(index);
    await _save();
    notifyListeners();
  }

  Future<void> resetToDefaults() async {
    _items = List.from(QuickAccessConfig.defaults);
    await _save();
    notifyListeners();
  }

  Future<void> _save() async {
    await StorageService.instance.setString(
      QuickAccessConfig.storageKey,
      QuickAccessConfig.serialize(_items),
    );
  }
}
