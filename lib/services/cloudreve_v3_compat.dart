import 'package:dio/dio.dart';

import '../core/utils/file_utils.dart';

/// Compatibility helpers for legacy Cloudreve V3 instances.
///
/// The rest of the app is written against Cloudreve V4-style endpoints and
/// payloads.  When the configured base URL points to `/api/v3`, this adapter
/// translates every API path used by the app into either a V3 endpoint, a V3
/// best-effort equivalent, or a synthetic successful response for V4-only
/// features that do not exist in Cloudreve V3 community/Pro.
class CloudreveV3Compat {
  const CloudreveV3Compat._();

  static bool isV3BaseUrl(String baseUrl) {
    final uri = Uri.tryParse(baseUrl.trim());
    if (uri == null) return false;
    return uri.pathSegments.length >= 2 &&
        uri.pathSegments[uri.pathSegments.length - 2] == 'api' &&
        uri.pathSegments.last.toLowerCase() == 'v3';
  }

  static String normalizeBaseUrl(String input) {
    final trimmed = input.trim().replaceAll(RegExp(r'/+$'), '');
    if (trimmed.contains('/api/v3') || trimmed.contains('/api/v4')) return trimmed;
    return '$trimmed/api/v3';
  }

  static void applyAuthHeader(RequestOptions options, String token) {
    final value = token.trim();
    if (value.isEmpty) return;
    options.headers['Cookie'] = value.contains('=') ? value : 'cloudreve-session=$value';
  }

  /// Returns a local response for V4-only APIs so V3 users do not hit 404s for
  /// optional UI sections (store, OAuth, WebDAV device management, versions,
  /// file viewers, etc.). Core auth/file/share/upload APIs are still sent to V3.
  static Response<dynamic>? syntheticResponse(RequestOptions options) {
    final path = options.path;
    final method = options.method.toUpperCase();

    if (path == '/session/prepare') {
      return _ok(options, {'password': true, 'webauthn': false, 'sso': false, 'two_factor': false});
    }
    if (path == '/session/token/2fa') {
      return _err(options, 40021, 'Cloudreve V3 API does not expose the V4 2FA token endpoint.');
    }
    if (path == '/session/user') {
      return _ok(options, <String, dynamic>{});
    }
    if (path.startsWith('/session/oauth/') || path == '/session/oidc/unlink') {
      return _ok(options, null);
    }

    if (path == '/devices/dav' || path.startsWith('/devices/dav/')) {
      if (method == 'GET') return _ok(options, <Map<String, dynamic>>[]);
      return _ok(options, null);
    }

    if (path.startsWith('/vas/') || path.startsWith('/site/config/vas') || path.startsWith('/site/config/payment') || path.startsWith('/site/config/shop') || path.startsWith('/site/config/store')) {
      return method == 'GET' ? _ok(options, <String, dynamic>{}) : _ok(options, null);
    }

    if (path.startsWith('/workflow')) {
      return _syntheticWorkflow(options);
    }

    if (path == '/user/setting' || path.startsWith('/user/setting/') || path == '/user/authn' || path == '/user/creditChanges') {
      if (method == 'GET') return _ok(options, path == '/user/creditChanges' ? <Map<String, dynamic>>[] : <String, dynamic>{});
      return _ok(options, null);
    }

    if (path == '/group/list') {
      return _ok(options, <Map<String, dynamic>>[]);
    }

    if (path.startsWith('/admin/')) {
      return method == 'GET' ? _ok(options, <String, dynamic>{}) : _ok(options, null);
    }

    if (path == '/file/lock' || (path == '/file/upload' && method == 'DELETE')) {
      return _ok(options, null);
    }

    if (path == '/file/version/current' || path == '/file/version' || path == '/file/viewerSession') {
      return _ok(options, path == '/file/viewerSession' ? <String, dynamic>{} : null);
    }

    return null;
  }

  static RequestOptions translateRequest(RequestOptions options) {
    final path = options.path;
    final method = options.method.toUpperCase();

    if (path == '/session/token') {
      if (method == 'DELETE') {
        options.path = '/user/session';
        return options;
      }
      options.path = '/user/session';
      final body = _body(options);
      options.data = {
        'userName': body['email'] ?? body['userName'] ?? body['username'],
        'Password': body['password'] ?? body['Password'],
        if ((body['captcha']?.toString().isNotEmpty ?? false)) 'captchaCode': body['captcha'],
        if ((body['ticket']?.toString().isNotEmpty ?? false)) 'captchaTicket': body['ticket'],
      };
    } else if (path == '/session/token/refresh') {
      options.extra['cloudreveV3RefreshToken'] = options.headers['Cookie'];
      options.path = '/user';
      options.method = 'GET';
      options.data = null;
    } else if (path == '/user/me') {
      options.path = '/user';
    } else if (path == '/user/capacity') {
      options.path = '/user/storage';
    } else if (path == '/user/reset' || path == '/user') {
      // V3 keeps compatible user register/reset endpoints in most builds.
    } else if (path.startsWith('/site/config')) {
      options.path = '/site/config';
    } else if (path == '/site/captcha') {
      // Same endpoint name in V3; response is normalized below.
    } else if (path == '/file' && method == 'GET') {
      final uri = options.queryParameters['uri']?.toString() ?? '/';
      final query = _cloudreveUriQuery(uri);
      final searchName = query['name']?.trim();
      if (searchName != null && searchName.isNotEmpty) {
        options.path = '/file/search/${Uri.encodeComponent(searchName)}';
        options.queryParameters = {
          if (options.queryParameters['page'] != null) 'page': options.queryParameters['page'],
          if (options.queryParameters['page_size'] != null) 'page_size': options.queryParameters['page_size'],
          'path': _uriToV3Path(uri),
        };
      } else {
        options.path = '/directory/${Uri.encodeComponent(_uriToV3Path(uri))}';
        options.queryParameters = {
          if (options.queryParameters['page'] != null) 'page': options.queryParameters['page'],
          if (options.queryParameters['page_size'] != null) 'page_size': options.queryParameters['page_size'],
        };
      }
    } else if (path == '/file/create') {
      final body = _body(options);
      if (body['type'] == 'folder') {
        final fullPath = _uriToV3Path(body['uri']?.toString() ?? '/');
        options.path = '/directory';
        options.method = 'PUT';
        options.data = {'path': _parentOf(fullPath), 'name': _nameOf(fullPath)};
      } else {
        options.path = '/file/create';
        options.data = {'path': _uriToV3Path(body['uri']?.toString() ?? '/')};
      }
    } else if (path == '/file' && method == 'DELETE') {
      final paths = _bodyUris(options).map(_uriToV3Path).toList();
      options.path = '/object';
      options.data = {'items': paths, 'dirs': paths};
    } else if (path == '/file/lock') {
      // V3 has no V4 lock API; avoid 404 in cleanup flows.
      options.extra['cloudreveV3ForceOk'] = true;
    } else if (path == '/file/rename') {
      final body = _body(options);
      final src = _uriToV3Path(body['uri']?.toString() ?? '/');
      options.path = '/object/rename';
      options.method = 'PUT';
      options.data = {'src': src, 'dst': '${_parentOf(src)}/${body['new_name']}'.replaceAll('//', '/')};
    } else if (path == '/file/move') {
      final body = _body(options);
      options.path = '/object';
      options.method = 'PATCH';
      options.data = {
        'src': (body['uris'] as List? ?? const []).map((e) => _uriToV3Path(e.toString())).toList(),
        'dst': _uriToV3Path(body['dst']?.toString() ?? '/'),
      };
    } else if (path == '/file/url') {
      options.path = '/file/download';
      options.data = {'items': _bodyUris(options).map(_uriToV3Path).toList()};
    } else if (path == '/file/source') {
      options.data = {'items': _bodyUris(options).map(_uriToV3Path).toList()};
    } else if (path == '/file/restore') {
      options.path = '/object/restore';
      options.data = {'items': _bodyUris(options).map(_uriToV3Path).toList()};
    } else if (path == '/file/info') {
      final uri = options.queryParameters['uri']?.toString() ?? options.queryParameters['id']?.toString() ?? '/';
      options.path = '/object/property/${Uri.encodeComponent(_uriToV3Path(uri))}';
      options.queryParameters = const {};
    } else if (path == '/file/thumb') {
      final uri = options.queryParameters['uri']?.toString() ?? options.queryParameters['path']?.toString() ?? '/';
      options.queryParameters = {'path': _uriToV3Path(uri)};
    } else if (path == '/file/upload') {
      if (method == 'PUT') {
        final body = _body(options);
        options.data = {'path': _uriToV3Path(body['uri']?.toString() ?? '/'), 'size': body['size'] ?? 0};
      } else if (method == 'DELETE') {
        options.extra['cloudreveV3ForceOk'] = true;
      }
    } else if (path.startsWith('/file/upload/')) {
      // V3 and V4 both use chunk upload paths. Keep as-is.
    } else if (path == '/share') {
      options.data = _translateShareBody(options.data);
    } else if (path.startsWith('/share/info/')) {
      // Same public info path in V3-like clients; response normalized below.
    } else if (RegExp(r'^/share/[^/]+$').hasMatch(path)) {
      // Same share detail/update/delete path family in V3 builds.
    } else if (path == '/user/search') {
      options.queryParameters = {'keywords': options.queryParameters['keywords'] ?? options.queryParameters['email'] ?? options.queryParameters['q'] ?? ''};
    }

    return options;
  }

  static dynamic translateResponse(Response<dynamic> response) {
    if (response.requestOptions.extra['cloudreveV3ForceOk'] == true) {
      return {'code': 0, 'msg': '', 'data': null};
    }

    final data = response.data;
    if (data is! Map) return data;
    final map = Map<String, dynamic>.from(data);
    final path = response.requestOptions.path;

    if (path == '/user/session') {
      final cookie = _sessionCookie(response);
      final user = Map<String, dynamic>.from(map['data'] as Map? ?? const {});
      map['data'] = {'user': _normalizeUser(user), 'token': _token(cookie)};
    } else if (path == '/user' && response.requestOptions.extra['cloudreveV3RefreshToken'] != null) {
      final cookie = response.requestOptions.extra['cloudreveV3RefreshToken']?.toString() ?? '';
      map['data'] = _token(cookie.replaceFirst(RegExp(r'^cloudreve-session='), ''));
    } else if (path == '/user') {
      map['data'] = _normalizeUser(Map<String, dynamic>.from(map['data'] as Map? ?? const {}));
    } else if (path == '/user/storage') {
      final raw = Map<String, dynamic>.from(map['data'] as Map? ?? const {});
      map['data'] = {'total': raw['total'] ?? raw['capacity'] ?? raw['space'] ?? 0, 'used': raw['used'] ?? raw['use'] ?? 0};
    } else if (path == '/site/config') {
      map['data'] = _normalizeSiteConfig(Map<String, dynamic>.from(map['data'] as Map? ?? const {}));
    } else if (path == '/site/captcha') {
      final raw = Map<String, dynamic>.from(map['data'] as Map? ?? const {});
      map['data'] = {'image': raw['image'] ?? raw['captcha'] ?? raw['src'] ?? '', 'ticket': raw['ticket'] ?? raw['captchaID'] ?? raw['id'] ?? ''};
    } else if (path.startsWith('/directory/') || path.startsWith('/file/search/')) {
      map['data'] = _normalizeDirectory(map['data']);
    } else if (path.startsWith('/object/property/')) {
      map['data'] = _normalizeObject(map['data']);
    } else if (path == '/file/download') {
      map['data'] = _normalizeDownloadUrls(map['data']);
    } else if (path == '/file/source') {
      map['data'] = _normalizeDirectLinks(map['data']);
    } else if (path == '/file/upload' && response.requestOptions.method.toUpperCase() == 'PUT') {
      map['data'] = _normalizeUploadSession(map['data']);
    } else if (path == '/share' || path.startsWith('/share/')) {
      map['data'] = _normalizeSharePayload(map['data']);
    } else if (path == '/user/search') {
      final raw = map['data'];
      final users = raw is List ? raw : (raw is Map ? (raw['users'] as List? ?? const []) : const []);
      map['data'] = users.map((e) => _normalizeUser(Map<String, dynamic>.from(e as Map? ?? const {}))).toList();
    }

    return map;
  }

  static Response<dynamic> _syntheticWorkflow(RequestOptions options) {
    final path = options.path;
    final method = options.method.toUpperCase();
    if (method == 'POST') return _ok(options, {'id': '', 'task_id': ''});
    if (path.startsWith('/workflow/progress/')) return _ok(options, {'status': 'completed', 'progress': 1.0});
    if (method == 'GET') return _ok(options, {'tasks': <Map<String, dynamic>>[], 'pagination': {'total_items': 0}});
    return _ok(options, null);
  }

  static Response<dynamic> _ok(RequestOptions options, dynamic data) => Response<dynamic>(
        requestOptions: options,
        statusCode: 200,
        data: {'code': 0, 'msg': '', 'data': data},
      );

  static Response<dynamic> _err(RequestOptions options, int code, String msg) => Response<dynamic>(
        requestOptions: options,
        statusCode: 200,
        data: {'code': code, 'msg': msg, 'data': null},
      );

  static Map<String, dynamic> _body(RequestOptions options) => Map<String, dynamic>.from(options.data as Map? ?? const {});

  static List<String> _bodyUris(RequestOptions options) => (_body(options)['uris'] as List? ?? const []).map((e) => e.toString()).toList();

  static String _sessionCookie(Response<dynamic> response) {
    final setCookie = response.headers.map['set-cookie']?.join('; ') ?? '';
    final match = RegExp(r'cloudreve-session=([^;]+)').firstMatch(setCookie);
    return match?.group(1) ?? setCookie;
  }

  static Map<String, dynamic> _token(String cookie) => {
        'access_token': cookie,
        'refresh_token': cookie,
        'access_expires': DateTime.now().add(const Duration(days: 30)).toIso8601String(),
        'refresh_expires': DateTime.now().add(const Duration(days: 30)).toIso8601String(),
      };

  static Map<String, dynamic> _normalizeUser(Map<String, dynamic> user) => {
        ...user,
        'id': user['id']?.toString() ?? user['uid']?.toString() ?? '',
        'email': user['email'] ?? user['user_name'] ?? user['username'] ?? user['nickname'] ?? '',
        'nickname': user['nickname'] ?? user['user_name'] ?? user['username'] ?? user['email'] ?? '',
        'avatar': user['avatar'] ?? '',
        'created_at': user['created_at'] ?? user['createdAt'] ?? DateTime.now().toIso8601String(),
        'group': user['group'] ?? {'id': '0', 'name': user['group_name'] ?? 'default'},
      };

  static Map<String, dynamic> _normalizeSiteConfig(Map<String, dynamic> cfg) => {
        ...cfg,
        'login_captcha': cfg['login_captcha'] ?? (cfg['captcha'] == true || cfg['captcha_login'] == true),
        'reg_captcha': cfg['reg_captcha'] ?? (cfg['captcha'] == true || cfg['captcha_reg'] == true),
        'forget_captcha': cfg['forget_captcha'] ?? (cfg['captcha'] == true),
        'themes': cfg['themes'] ?? <Map<String, dynamic>>[],
        'storage_products': cfg['storage_products'] ?? <Map<String, dynamic>>[],
        'file_viewers': cfg['file_viewers'] ?? <Map<String, dynamic>>[],
      };

  static Map<String, dynamic> _normalizeDirectory(dynamic raw) {
    final payload = Map<String, dynamic>.from(raw as Map? ?? const {});
    final objects = (payload['objects'] as List?) ?? (payload['files'] as List?) ?? const [];
    return {
      'files': objects.map(_normalizeObject).toList(),
      'pagination': {'total_items': payload['total'] ?? payload['total_items'] ?? objects.length},
      'storage_policy': payload['policy'] ?? payload['storage_policy'],
    };
  }

  static Map<String, dynamic> _normalizeObject(dynamic value) {
    final obj = Map<String, dynamic>.from(value as Map? ?? const {});
    final path = obj['path']?.toString() ?? obj['source']?.toString() ?? '/${obj['name'] ?? ''}';
    final isDir = obj['type'] == 'dir' || obj['type'] == 1 || obj['is_dir'] == true || obj['is_folder'] == true;
    return {
      ...obj,
      'id': obj['id']?.toString() ?? path,
      'name': obj['name'] ?? _nameOf(path),
      'path': path,
      'uri': FileUtils.toCloudreveUri(path),
      'size': obj['size'] ?? 0,
      'type': isDir ? 1 : 0,
      'created_at': obj['date'] ?? obj['created_at'] ?? obj['createdAt'] ?? '',
      'updated_at': obj['date'] ?? obj['updated_at'] ?? obj['updatedAt'] ?? '',
      'source_enabled': obj['source_enabled'] ?? obj['sourceEnabled'] ?? false,
    };
  }

  static Map<String, dynamic> _normalizeDownloadUrls(dynamic raw) {
    final urls = raw is List
        ? raw
        : raw is Map && raw['urls'] is List
            ? raw['urls'] as List
            : [raw];
    return {'urls': urls.map((e) => e is Map ? {'url': e['url']?.toString() ?? ''} : {'url': e?.toString() ?? ''}).toList()};
  }

  static List<Map<String, dynamic>> _normalizeDirectLinks(dynamic raw) {
    final links = raw is List
        ? raw
        : raw is Map && raw['links'] is List
            ? raw['links'] as List
            : raw is Map && raw['urls'] is List
                ? raw['urls'] as List
                : raw == null
                    ? const []
                    : [raw];
    return links.map((e) {
      final item = e is Map ? Map<String, dynamic>.from(e) : <String, dynamic>{'url': e?.toString() ?? ''};
      return {
        ...item,
        'url': item['url']?.toString() ?? item['src']?.toString() ?? item['source']?.toString() ?? '',
      };
    }).toList();
  }

  static Map<String, dynamic> _normalizeUploadSession(dynamic raw) {
    final payload = Map<String, dynamic>.from(raw as Map? ?? const {});
    final policy = Map<String, dynamic>.from((payload['policy'] as Map?) ?? (payload['storage_policy'] as Map?) ?? const {});
    return {
      ...payload,
      'session_id': payload['sessionID'] ?? payload['session_id'] ?? payload['id'] ?? '',
      'chunk_size': payload['chunkSize'] ?? payload['chunk_size'] ?? 10 * 1024 * 1024,
      'upload_urls': payload['uploadURLs'] ?? payload['upload_urls'] ?? payload['uploadUrls'],
      'storage_policy': {
        ...policy,
        'type': policy['type'] ?? policy['policy_type'] ?? 'local',
        'relay': policy['relay'] ?? true,
      },
      'expires': payload['expires'] ?? 0,
    };
  }

  static dynamic _normalizeSharePayload(dynamic raw) {
    if (raw is List) return raw.map(_normalizeShare).toList();
    if (raw is Map && raw['shares'] is List) {
      return {...raw, 'shares': (raw['shares'] as List).map(_normalizeShare).toList()};
    }
    if (raw is Map) return _normalizeShare(raw);
    return raw;
  }

  static Map<String, dynamic> _normalizeShare(dynamic value) {
    final share = Map<String, dynamic>.from(value as Map? ?? const {});
    return {
      ...share,
      'id': share['id']?.toString() ?? share['key']?.toString() ?? '',
      'uri': share['uri'] ?? FileUtils.toCloudreveUri(share['source']?.toString() ?? '/'),
      'created_at': share['created_at'] ?? share['create_date'] ?? share['createdAt'] ?? '',
    };
  }

  static dynamic _translateShareBody(dynamic data) {
    if (data is! Map) return data;
    final body = Map<String, dynamic>.from(data);
    return {
      ...body,
      if (body['uri'] != null) 'path': _uriToV3Path(body['uri'].toString()),
      if (body['uris'] is List) 'items': (body['uris'] as List).map((e) => _uriToV3Path(e.toString())).toList(),
    };
  }

  static String _uriToV3Path(String uri) {
    var path = FileUtils.toCloudreveUri(uri).split('?').first;
    path = path.replaceFirst(RegExp(r'^cloudreve://my'), '');
    path = path.replaceFirst(RegExp(r'^cloudreve://trash'), '/');
    if (path.isEmpty) return '/';
    return path.startsWith('/') ? path : '/$path';
  }

  static Map<String, String> _cloudreveUriQuery(String uri) {
    final idx = uri.indexOf('?');
    if (idx < 0 || idx == uri.length - 1) return const {};
    return Uri.splitQueryString(uri.substring(idx + 1));
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
