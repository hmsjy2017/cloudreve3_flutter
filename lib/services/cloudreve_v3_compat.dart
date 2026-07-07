import 'package:dio/dio.dart';

import '../core/utils/file_utils.dart';

/// Compatibility helpers for legacy Cloudreve V3 instances.
///
/// Cloudreve V4 uses JWT/OAuth style APIs under `/api/v4`, while V3 uses the
/// older `/api/v3` API and session cookies.  This adapter keeps most of the app
/// talking in the existing V4-shaped models and translates the commonly used
/// auth/file endpoints when the configured server URL contains `/api/v3`.
class CloudreveV3Compat {
  const CloudreveV3Compat._();

  static bool isV3BaseUrl(String baseUrl) => Uri.tryParse(baseUrl)?.path.contains('/api/v3') ?? false;

  static String normalizeBaseUrl(String input) {
    final trimmed = input.trim().replaceAll(RegExp(r'/+$'), '');
    if (trimmed.contains('/api/v3') || trimmed.contains('/api/v4')) return trimmed;
    return '$trimmed/api/v3';
  }

  static void applyAuthHeader(RequestOptions options, String token) {
    final value = token.trim();
    if (value.isEmpty) return;
    if (value.contains('=')) {
      options.headers['Cookie'] = value;
    } else {
      options.headers['Cookie'] = 'cloudreve-session=$value';
    }
  }

  static RequestOptions translateRequest(RequestOptions options) {
    final path = options.path;

    if (path == '/session/token') {
      options.path = '/user/session';
      final body = Map<String, dynamic>.from(options.data as Map? ?? const {});
      options.data = {
        'userName': body['email'] ?? body['userName'] ?? body['username'],
        'Password': body['password'] ?? body['Password'],
        if ((body['captcha']?.toString().isNotEmpty ?? false)) 'captchaCode': body['captcha'],
        if ((body['ticket']?.toString().isNotEmpty ?? false)) 'captchaTicket': body['ticket'],
      };
    } else if (path == '/session/token/refresh') {
      // V3 session cookies cannot be refreshed through a refresh-token API; the
      // saved cookie is reused until the server invalidates it.
      options.extra['cloudreveV3RefreshToken'] = options.headers['Cookie'];
      options.path = '/user';
      options.method = 'GET';
      options.data = null;
    } else if (path == '/user/me') {
      options.path = '/user';
    } else if (path == '/user/capacity') {
      options.path = '/user/storage';
    } else if (path == '/site/config/login' || path == '/site/config/basic') {
      options.path = '/site/config';
    } else if (path == '/file' && options.method.toUpperCase() == 'GET') {
      final uri = options.queryParameters['uri']?.toString() ?? '/';
      options.path = '/directory/${Uri.encodeComponent(_uriToV3Path(uri))}';
      options.queryParameters = {
        if (options.queryParameters['page'] != null) 'page': options.queryParameters['page'],
        if (options.queryParameters['page_size'] != null) 'page_size': options.queryParameters['page_size'],
      };
    } else if (path == '/file/create') {
      final body = Map<String, dynamic>.from(options.data as Map? ?? const {});
      if (body['type'] == 'folder') {
        final fullPath = _uriToV3Path(body['uri']?.toString() ?? '/');
        options.path = '/directory';
        options.method = 'PUT';
        options.data = {
          'path': _parentOf(fullPath),
          'name': _nameOf(fullPath),
        };
      }
    } else if (path == '/file' && options.method.toUpperCase() == 'DELETE') {
      final body = Map<String, dynamic>.from(options.data as Map? ?? const {});
      final paths = (body['uris'] as List? ?? const []).map((e) => _uriToV3Path(e.toString())).toList();
      options.path = '/object';
      options.data = {'items': paths, 'dirs': paths};
    } else if (path == '/file/rename') {
      final body = Map<String, dynamic>.from(options.data as Map? ?? const {});
      final src = _uriToV3Path(body['uri']?.toString() ?? '/');
      options.path = '/object/rename';
      options.method = 'PUT';
      options.data = {'src': src, 'dst': '${_parentOf(src)}/${body['new_name']}'.replaceAll('//', '/')};
    } else if (path == '/file/move') {
      final body = Map<String, dynamic>.from(options.data as Map? ?? const {});
      final paths = (body['uris'] as List? ?? const []).map((e) => _uriToV3Path(e.toString())).toList();
      options.path = '/object';
      options.method = 'PATCH';
      options.data = {'src': paths, 'dst': _uriToV3Path(body['dst']?.toString() ?? '/')};
    } else if (path == '/file/url') {
      final body = Map<String, dynamic>.from(options.data as Map? ?? const {});
      options.path = '/file/download';
      options.data = {'items': (body['uris'] as List? ?? const []).map((e) => _uriToV3Path(e.toString())).toList()};
    }

    return options;
  }

  static dynamic translateResponse(Response<dynamic> response) {
    final data = response.data;
    if (data is! Map) return data;
    final map = Map<String, dynamic>.from(data);
    final path = response.requestOptions.path;

    if (path == '/user/session') {
      final cookie = _sessionCookie(response);
      final user = Map<String, dynamic>.from(map['data'] as Map? ?? const {});
      map['data'] = {
        'user': _normalizeUser(user),
        'token': {
          'access_token': cookie,
          'refresh_token': cookie,
          'access_expires': DateTime.now().add(const Duration(days: 30)).toIso8601String(),
          'refresh_expires': DateTime.now().add(const Duration(days: 30)).toIso8601String(),
        },
      };
    } else if (path == '/user' && response.requestOptions.extra['cloudreveV3RefreshToken'] != null) {
      final cookie = response.requestOptions.extra['cloudreveV3RefreshToken']?.toString() ?? '';
      map['data'] = {
        'access_token': cookie.replaceFirst(RegExp(r'^cloudreve-session='), ''),
        'refresh_token': cookie.replaceFirst(RegExp(r'^cloudreve-session='), ''),
        'access_expires': DateTime.now().add(const Duration(days: 30)).toIso8601String(),
        'refresh_expires': DateTime.now().add(const Duration(days: 30)).toIso8601String(),
      };
    } else if (path == '/user') {
      map['data'] = _normalizeUser(Map<String, dynamic>.from(map['data'] as Map? ?? const {}));
    } else if (path.startsWith('/directory/')) {
      final payload = Map<String, dynamic>.from(map['data'] as Map? ?? const {});
      final objects = payload['objects'] as List? ?? const [];
      map['data'] = {
        'files': objects.map(_normalizeObject).toList(),
        'pagination': {'total_items': payload['total'] ?? objects.length},
        'storage_policy': payload['policy'],
      };
    } else if (path == '/file/download') {
      final raw = map['data'];
      final urls = raw is List ? raw : [raw];
      map['data'] = {'urls': urls.map((e) => {'url': e?.toString() ?? ''}).toList()};
    } else if (path == '/site/config') {
      final cfg = Map<String, dynamic>.from(map['data'] as Map? ?? const {});
      map['data'] = {
        ...cfg,
        'login_captcha': cfg['captcha'] == true || cfg['captcha_login'] == true,
        'reg_captcha': cfg['captcha'] == true || cfg['captcha_reg'] == true,
      };
    }

    return map;
  }

  static String _sessionCookie(Response<dynamic> response) {
    final setCookie = response.headers.map['set-cookie']?.join('; ') ?? '';
    final match = RegExp(r'cloudreve-session=([^;]+)').firstMatch(setCookie);
    return match?.group(1) ?? setCookie;
  }

  static Map<String, dynamic> _normalizeUser(Map<String, dynamic> user) => {
        ...user,
        'id': user['id']?.toString() ?? user['uid']?.toString() ?? '',
        'email': user['email'] ?? user['user_name'] ?? user['nickname'] ?? '',
        'nickname': user['nickname'] ?? user['user_name'] ?? user['email'] ?? '',
        'avatar': user['avatar'] ?? '',
        'created_at': user['created_at'] ?? DateTime.now().toIso8601String(),
        'group': user['group'] ?? {'id': '0', 'name': user['group_name'] ?? 'default'},
      };

  static Map<String, dynamic> _normalizeObject(dynamic value) {
    final obj = Map<String, dynamic>.from(value as Map? ?? const {});
    final path = obj['path']?.toString() ?? '/${obj['name'] ?? ''}';
    final isDir = obj['type'] == 'dir' || obj['type'] == 1 || obj['is_dir'] == true;
    return {
      ...obj,
      'id': obj['id']?.toString() ?? path,
      'name': obj['name'] ?? _nameOf(path),
      'path': path,
      'uri': FileUtils.toCloudreveUri(path),
      'size': obj['size'] ?? 0,
      'type': isDir ? 1 : 0,
      'created_at': obj['date'] ?? obj['created_at'] ?? '',
      'updated_at': obj['date'] ?? obj['updated_at'] ?? '',
    };
  }

  static String _uriToV3Path(String uri) {
    var path = FileUtils.toCloudreveUri(uri);
    path = path.replaceFirst(RegExp(r'^cloudreve://my'), '');
    path = path.replaceFirst(RegExp(r'^cloudreve://trash'), '/');
    if (path.isEmpty) return '/';
    return path.startsWith('/') ? path : '/$path';
  }

  static String _parentOf(String path) {
    final idx = path.lastIndexOf('/');
    if (idx <= 0) return '/';
    return path.substring(0, idx);
  }

  static String _nameOf(String path) {
    final clean = path.replaceAll(RegExp(r'/+$'), '');
    final idx = clean.lastIndexOf('/');
    return idx < 0 ? clean : clean.substring(idx + 1);
  }
}
