import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_flutter/webview_flutter.dart' as mobile;

import '../../../core/utils/app_logger.dart';
import '../../../services/api_service.dart';
import '../../../services/share_link_service.dart';
import '../../providers/download_manager_provider.dart';
import '../../widgets/folder_picker.dart';
import '../../widgets/user_avatar.dart';

bool get _isDesktop => !kIsWeb && (Platform.isWindows || Platform.isLinux);

class ShareLinkPage extends StatefulWidget {
  final ShareLinkCandidate candidate;

  const ShareLinkPage({
    super.key,
    required this.candidate,
  });

  @override
  State<ShareLinkPage> createState() => _ShareLinkPageState();
}

class _ShareLinkPageState extends State<ShareLinkPage> {
  final TextEditingController _passwordController = TextEditingController();
  final List<_ShareBreadcrumb> _breadcrumbs = <_ShareBreadcrumb>[];

  ShareLinkInfo? _info;
  ShareLinkFile? _singleFile;
  List<ShareLinkFile> _files = const [];
  Object? _error;
  Object? _fileError;
  String? _contextHint;
  String? _currentUri;
  bool _loadingInfo = true;
  bool _loadingFiles = false;
  bool _openingDownload = false;
  bool _saving = false;

  String get _displayUrl => _info?.url ?? widget.candidate.url;

  bool get _isSameOrigin {
    final shareUri = Uri.tryParse(widget.candidate.url);
    final baseUri = Uri.tryParse(ApiService.instance.dio.options.baseUrl);
    if (shareUri == null || baseUri == null) return false;
    final same = shareUri.host == baseUri.host;
    AppLogger.d('ShareLinkPage _isSameOrigin: shareHost=${shareUri.host}, baseHost=${baseUri.host}, result=$same');
    return same;
  }

  @override
  void initState() {
    super.initState();
    _passwordController.text = widget.candidate.password ?? '';
    _loadShare(password: widget.candidate.password);
  }

  @override
  void dispose() {
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _loadShare({String? password}) async {
    setState(() {
      _loadingInfo = true;
      _error = null;
      _fileError = null;
      _files = const [];
      _singleFile = null;
      _contextHint = null;
      _currentUri = null;
      _breadcrumbs.clear();
    });

    try {
      final info = await ShareLinkService.instance.getShareInfo(
        widget.candidate,
        password: password?.trim().isEmpty == true ? null : password,
      );

      if (!mounted) return;

      setState(() {
        _info = info;
        _contextHint = info.contextHint ?? _contextHint;
        _loadingInfo = false;
      });

      final sourceUri = info.sourceUri;
      if (info.expired || !info.unlocked || sourceUri == null) return;

      if (info.isFolder) {
        await _loadFolder(sourceUri, resetBreadcrumbs: true, title: info.name);
      } else {
        await _loadSingleFile(sourceUri);
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e;
        _loadingInfo = false;
      });
    }
  }

  Future<void> _loadSingleFile(String uri) async {
    setState(() {
      _loadingFiles = true;
      _fileError = null;
      _currentUri = uri;
    });

    // Cloudreve 官方分享页会把 /s/{id} 重定向到
    // /home?path=cloudreve://{id}@share，然后按普通文件列表读取 share 文件系统。
    // 单文件分享也要先读取 share 根目录，拿到服务端返回的真实文件 path/context_hint，
    // 再用这个 path 调 /file/url。直接拿 share 根 URI 下载会得到 40081。
    try {
      final list = await ShareLinkService.instance.listSharedFiles(
        uri: uri,
        contextHint: _contextHint,
      );
      if (list.files.isNotEmpty) {
        final file = list.files.first;
        if (!mounted) return;
        setState(() {
          _singleFile = file;
          _contextHint = file.contextHint ?? list.contextHint ?? _contextHint;
          _currentUri = file.path.isNotEmpty ? file.path : uri;
          _loadingFiles = false;
        });
        return;
      }
      if (list.contextHint != null && list.contextHint!.isNotEmpty) {
        _contextHint = list.contextHint;
      }
    } catch (_) {
      // 某些服务端不允许对单文件分享根目录执行 /file，继续走 /file/info 降级。
    }

    try {
      final file = await ShareLinkService.instance.getSharedFileInfo(
        uri: uri,
        contextHint: _contextHint,
        shareId: widget.candidate.id,
        password: widget.candidate.password,
      );

      if (!mounted) return;
      setState(() {
        _singleFile = file;
        _contextHint = file.contextHint ?? _contextHint;
        _currentUri = file.path.isNotEmpty ? file.path : uri;
        _loadingFiles = false;
      });
    } catch (e) {
      // 部分服务端在公开分享上下文中不允许读取 /file/info，
      // 但仍允许通过 /file/url 下载或 /file/move 转存。这里降级为分享信息卡片，
      // 避免页面出现大块错误提示。
      final info = _info;
      if (!mounted) return;
      setState(() {
        _singleFile = info == null ? null : ShareLinkService.instance.fileFromShareInfo(info);
        _contextHint = _singleFile?.contextHint ?? _contextHint ?? info?.contextHint;
        _fileError = null;
        _loadingFiles = false;
      });
    }
  }

  Future<void> _loadFolder(
    String uri, {
    bool resetBreadcrumbs = false,
    String? title,
  }) async {
    setState(() {
      _loadingFiles = true;
      _fileError = null;
      _currentUri = uri;
      if (resetBreadcrumbs) {
        _breadcrumbs
          ..clear()
          ..add(_ShareBreadcrumb(title: title ?? '分享目录', uri: uri));
      }
    });

    try {
      final result = await ShareLinkService.instance.listSharedFiles(
        uri: uri,
        contextHint: _contextHint,
      );

      if (!mounted) return;
      setState(() {
        _files = result.files;
        _contextHint = result.contextHint ?? _contextHint;
        _loadingFiles = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _fileError = e;
        _loadingFiles = false;
      });
    }
  }

  Future<void> _refresh() async {
    final info = _info;
    if (info == null) {
      await _loadShare(password: _passwordController.text.trim());
      return;
    }

    final uri = _currentUri ?? info.sourceUri;
    if (uri == null || !info.unlocked || info.expired) {
      await _loadShare(password: _passwordController.text.trim());
      return;
    }

    if (info.isFolder) {
      await _loadFolder(uri);
    } else {
      await _loadSingleFile(uri);
    }
  }

  Future<void> _openExternal() async {
    final uri = Uri.tryParse(_displayUrl);
    if (uri == null) return;
    await launchUrl(uri, mode: LaunchMode.externalApplication);
  }

  Future<void> _openDownloadUrl(
    String uri, {
    required String fileName,
    required int fileSize,
    String? entity,
    bool archive = false,
  }) async {
    if (_openingDownload) return;

    setState(() => _openingDownload = true);
    try {
      final result = await ShareLinkService.instance.createShareDownloadUrl(
        uri: uri,
        contextHint: _contextHint,
        shareId: widget.candidate.id,
        password: widget.candidate.password,
        entity: entity,
        fileName: fileName,
        archive: archive,
      );

      await _enqueueInAppDownload(
        result.url,
        fileName: fileName,
        fileSize: fileSize,
        fileUri: uri,
      );
    } catch (e) {
      if (!mounted) return;

      final fallbackUrl = await _obtainDownloadUrlFromOfficialPage(
        fileName: fileName,
      );
      if (fallbackUrl != null && fallbackUrl.isNotEmpty) {
        try {
          await _enqueueInAppDownload(
            fallbackUrl,
            fileName: fileName,
            fileSize: fileSize,
            fileUri: uri,
          );
          return;
        } catch (downloadError) {
          if (!mounted) return;
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('创建下载任务失败：$downloadError')),
          );
          return;
        }
      }

      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('获取下载链接失败：$e'),
          action: SnackBarAction(label: '浏览器打开', onPressed: _openExternal),
        ),
      );
    } finally {
      if (mounted) setState(() => _openingDownload = false);
    }
  }

  Future<void> _enqueueInAppDownload(
    String rawUrl, {
    required String fileName,
    required int fileSize,
    required String fileUri,
  }) async {
    final downloadUri = Uri.tryParse(rawUrl);
    if (downloadUri == null || !downloadUri.hasScheme) {
      throw Exception('下载链接格式错误');
    }

    final task = await context.read<DownloadManagerProvider>().addDownloadTask(
          fileName: fileName,
          fileUri: fileUri,
          fileSize: fileSize,
        );

    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(task == null ? '该文件已在下载队列中' : '已添加到下载队列，正在下载'),
      ),
    );
  }


  Future<String?> _obtainDownloadUrlFromOfficialPage({
    required String fileName,
  }) async {
    final shareUri = Uri.tryParse(widget.candidate.url);
    if (shareUri == null) return null;

    final completer = Completer<String?>();
    BuildContext? dialogContext;
    Timer? timeoutTimer;
    InAppWebViewController? desktopCtrl;

    void complete(String? url) {
      if (completer.isCompleted) return;
      completer.complete(url);
    }

    timeoutTimer = Timer(const Duration(seconds: 24), () => complete(null));

    completer.future.then((url) {
      final ctx = dialogContext;
      if (!mounted && !context.mounted) return;
      if (ctx != null && Navigator.of(ctx, rootNavigator: true).canPop()) {
        Navigator.of(ctx, rootNavigator: true).pop(url);
      }
    });

    Widget webViewWidget;

    if (!_isDesktop) {
      // ── 移动端：webview_flutter ──
      late mobile.WebViewController controller;
      controller = mobile.WebViewController()
        ..setJavaScriptMode(mobile.JavaScriptMode.unrestricted)
        ..addJavaScriptChannel(
          'CloudreveDownloadBridge',
          onMessageReceived: (message) {
            final url = _extractOfficialDownloadUrl(
              message.message,
              shareUri: shareUri,
            );
            if (url != null && url.isNotEmpty) {
              complete(url);
            }
          },
        )
        ..setNavigationDelegate(
          mobile.NavigationDelegate(
            onPageFinished: (_) async {
              await _installOfficialDownloadHook(
                controller.runJavaScript,
              );
            },
            onNavigationRequest: (request) {
              final url = request.url;
              if (_looksLikeDirectDownloadUrl(url, shareUri: shareUri)) {
                complete(url);
                return mobile.NavigationDecision.prevent;
              }
              return mobile.NavigationDecision.navigate;
            },
            onUrlChange: (change) {
              final url = change.url;
              if (url != null && _looksLikeDirectDownloadUrl(url, shareUri: shareUri)) {
                complete(url);
              }
            },
          ),
        )
        ..loadRequest(shareUri);

      webViewWidget = mobile.WebViewWidget(controller: controller);
    } else {
      // ── 桌面端：flutter_inappwebview ──
      webViewWidget = SizedBox(
        width: 1,
        height: 1,
        child: InAppWebView(
          initialSettings: InAppWebViewSettings(
            javaScriptEnabled: true,
            domStorageEnabled: true,
            isInspectable: true,
            cacheMode: CacheMode.LOAD_DEFAULT,
            supportMultipleWindows: false,
            useHybridComposition: true,
          ),
          onWebViewCreated: (controller) {
            desktopCtrl = controller;
            controller.addJavaScriptHandler(
              handlerName: 'CloudreveDownloadBridge',
              callback: (args) {
                final text = args.isNotEmpty ? args[0].toString() : '';
                final url = _extractOfficialDownloadUrl(
                  text,
                  shareUri: shareUri,
                );
                if (url != null && url.isNotEmpty) {
                  complete(url);
                }
              },
            );
            controller.loadUrl(
              urlRequest: URLRequest(url: WebUri(shareUri.toString())),
            );
          },
          onLoadStop: (controller, url) async {
            await _installOfficialDownloadHook(
              (script) => controller.evaluateJavascript(source: script),
            );
          },
          shouldInterceptRequest: (controller, request) async {
            final url = request.url.toString();
            if (_looksLikeDirectDownloadUrl(url, shareUri: shareUri)) {
              complete(url);
            }
            return null;
          },
          onLoadStart: (controller, url) {
            if (url != null && _looksLikeDirectDownloadUrl(url.toString(), shareUri: shareUri)) {
              complete(url.toString());
            }
          },
          onReceivedError: (controller, request, error) {
            // 忽略子资源错误
          },
        ),
      );
    }

    final result = await showDialog<String?>(
      context: context,
      barrierDismissible: false,
      builder: (context) {
        dialogContext = context;
        final theme = Theme.of(context);
        return AlertDialog(
          title: const Text('正在获取下载链接'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                '正在按 Cloudreve 官方分享页面流程获取「$fileName」的真实下载地址，获取成功后会加入应用内下载任务。',
                style: theme.textTheme.bodyMedium,
              ),
              const SizedBox(height: 14),
              const LinearProgressIndicator(),
              const SizedBox(height: 10),
              Text(
                '不会自动跳转浏览器；浏览器入口只作为手动备用方案。',
                style: theme.textTheme.bodySmall?.copyWith(color: theme.hintColor),
              ),
              const SizedBox(height: 1),
              SizedBox(
                width: 1,
                height: 1,
                child: Opacity(
                  opacity: 0.01,
                  child: webViewWidget,
                ),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => complete(null),
              child: const Text('取消'),
            ),
            TextButton(
              onPressed: _openExternal,
              child: const Text('浏览器打开'),
            ),
          ],
        );
      },
    );

    timeoutTimer.cancel();
    desktopCtrl?.dispose();
    return result;
  }

  Future<void> _installOfficialDownloadHook(Future<void> Function(String) runJS) async {
    const hookScript = r"""
(function () {
  if (window.__cloudreveAppDownloadHookInstalled) return;
  window.__cloudreveAppDownloadHookInstalled = true;

  function post(value) {
    try {
      if (typeof value !== 'string') value = JSON.stringify(value);
      if (typeof CloudreveDownloadBridge !== 'undefined' && CloudreveDownloadBridge.postMessage) {
        CloudreveDownloadBridge.postMessage(value);
      } else if (window.flutter_inappwebview && window.flutter_inappwebview.callHandler) {
        window.flutter_inappwebview.callHandler('CloudreveDownloadBridge', value);
      }
    } catch (e) {}
  }

  function inspectText(text) {
    if (!text) return;
    try {
      post(text);
    } catch (e) {}
  }

  var rawFetch = window.fetch;
  if (rawFetch) {
    window.fetch = function () {
      var req = arguments[0];
      var reqUrl = '';
      try { reqUrl = String((req && req.url) || req || ''); } catch (e) {}
      return rawFetch.apply(this, arguments).then(function (resp) {
        try {
          var respUrl = String(resp && resp.url ? resp.url : reqUrl);
          if (respUrl.indexOf('/api/v4/file/url') >= 0 || reqUrl.indexOf('/api/v4/file/url') >= 0) {
            resp.clone().text().then(inspectText).catch(function () {});
          }
        } catch (e) {}
        return resp;
      });
    };
  }

  var RawXHR = window.XMLHttpRequest;
  if (RawXHR) {
    var rawXHROpen = RawXHR.prototype.open;
    var rawXHRSend = RawXHR.prototype.send;
    RawXHR.prototype.open = function (method, url) {
      this.__cloudreveRequestUrl = String(url || '');
      return rawXHROpen.apply(this, arguments);
    };
    RawXHR.prototype.send = function () {
      try {
        this.addEventListener('load', function () {
          try {
            var url = String(this.responseURL || this.__cloudreveRequestUrl || '');
            if (url.indexOf('/api/v4/file/url') >= 0) {
              inspectText(String(this.responseText || ''));
            }
          } catch (e) {}
        });
      } catch (e) {}
      return rawXHRSend.apply(this, arguments);
    };
  }

  var rawOpen = window.open;
  window.open = function (url) {
    try { post({ __navigation_url: String(url || '') }); } catch (e) {}
    return rawOpen ? rawOpen.apply(this, arguments) : null;
  };

  document.addEventListener('click', function (event) {
    try {
      var target = event.target;
      var href = '';
      while (target && !href) {
        href = target.href || target.getAttribute && target.getAttribute('href') || '';
        target = target.parentElement;
      }
      if (href) post({ __navigation_url: String(href) });
    } catch (e) {}
  }, true);
})();
""";

    const clickScript = r"""
(function () {
  function visible(el) {
    if (!el) return false;
    var style = window.getComputedStyle(el);
    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
    var rect = el.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  }

  function clickDownload() {
    var nodes = Array.prototype.slice.call(document.querySelectorAll('button,a,[role="button"],div,span'));
    var best = null;
    for (var i = 0; i < nodes.length; i++) {
      var el = nodes[i];
      var text = [
        el.innerText || '',
        el.textContent || '',
        el.getAttribute && el.getAttribute('aria-label') || '',
        el.getAttribute && el.getAttribute('title') || '',
        el.className || ''
      ].join(' ').trim();
      if (!text || (text.indexOf('下载') < 0 && text.toLowerCase().indexOf('download') < 0)) continue;
      if (el.disabled || el.getAttribute('aria-disabled') === 'true') continue;
      best = el;
      break;
    }
    if (best) {
      try {
        best.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
        best.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true }));
        best.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      } catch (e) {
        best.click();
      }
      return true;
    }
    return false;
  }

  var tries = 0;
  var timer = setInterval(function () {
    tries++;
    if (clickDownload() || tries > 30) {
      clearInterval(timer);
    }
  }, 500);
})();
""";

    try {
      await runJS(hookScript);
      await runJS(clickScript);
    } catch (_) {
      // 官方页面脚本注入失败时，外层会超时并保留“浏览器打开”。
    }
  }

  String? _extractOfficialDownloadUrl(String message, {required Uri shareUri}) {
    final text = message.trim();
    if (text.isEmpty) return null;

    final direct = _normalizeOfficialUrl(text, shareUri: shareUri);
    if (direct != null && _looksLikeDirectDownloadUrl(direct, shareUri: shareUri)) {
      return direct;
    }

    try {
      final decoded = jsonDecode(text);
      return _walkOfficialPayloadForUrl(decoded, shareUri: shareUri);
    } catch (_) {
      return null;
    }
  }

  String? _walkOfficialPayloadForUrl(dynamic value, {required Uri shareUri}) {
    if (value == null) return null;

    if (value is String) {
      final normalized = _normalizeOfficialUrl(value, shareUri: shareUri);
      if (normalized != null && _looksLikeDirectDownloadUrl(normalized, shareUri: shareUri)) {
        return normalized;
      }
      return null;
    }

    if (value is List) {
      for (final item in value) {
        final url = _walkOfficialPayloadForUrl(item, shareUri: shareUri);
        if (url != null) return url;
      }
      return null;
    }

    if (value is Map) {
      final navigationUrl = value['__navigation_url'];
      if (navigationUrl is String) {
        final normalized = _normalizeOfficialUrl(navigationUrl, shareUri: shareUri);
        if (normalized != null && _looksLikeDirectDownloadUrl(normalized, shareUri: shareUri)) {
          return normalized;
        }
      }

      const priorityKeys = [
        'url',
        'download_url',
        'downloadUrl',
        'href',
        'src',
        'signed_url',
        'signedUrl',
        'link',
        'urls',
        'data',
      ];

      for (final key in priorityKeys) {
        if (!value.containsKey(key)) continue;
        final url = _walkOfficialPayloadForUrl(value[key], shareUri: shareUri);
        if (url != null) return url;
      }

      for (final item in value.values) {
        final url = _walkOfficialPayloadForUrl(item, shareUri: shareUri);
        if (url != null) return url;
      }
    }

    return null;
  }

  String? _normalizeOfficialUrl(String value, {required Uri shareUri}) {
    final text = value.trim();
    if (text.isEmpty) return null;
    final uri = Uri.tryParse(text);
    if (uri == null) return null;

    if (uri.hasScheme && (uri.scheme == 'http' || uri.scheme == 'https')) {
      return uri.toString();
    }

    if (text.startsWith('/')) {
      return shareUri.replace(path: text, query: '', fragment: '').toString();
    }

    return null;
  }

  bool _looksLikeDirectDownloadUrl(String value, {required Uri shareUri}) {
    final uri = Uri.tryParse(value);
    if (uri == null || !uri.hasScheme) return false;
    if (uri.scheme != 'http' && uri.scheme != 'https') return false;

    final sameHost = uri.host == shareUri.host;
    final path = uri.path.toLowerCase();

    if (sameHost) {
      if (path.startsWith('/s/') || path == '/home' || path.startsWith('/home/')) {
        return false;
      }
      if (path.contains('/api/v4/file/url')) return false;
      if (path.contains('/api/v4/share/info')) return false;
    }

    final url = uri.toString().toLowerCase();
    return !sameHost ||
        path.contains('/download') ||
        path.contains('/api/v4/file/download') ||
        path.contains('/api/v4/file/source') ||
        url.contains('response-content-disposition') ||
        url.contains('x-amz-signature') ||
        url.contains('x-oss-signature') ||
        url.contains('signature=') ||
        url.contains('sign=') ||
        url.contains('token=');
  }

  Future<void> _saveSharedUri(String uri, {required String name}) async {
    if (_saving) return;

    final destination = await _pickDestination();
    if (destination == null) return;

    setState(() => _saving = true);
    try {
      final isRootShareFolder = ShareLinkService.isShareRootUri(
            uri,
            shareId: widget.candidate.id,
          ) &&
          _files.isNotEmpty;
      final urisToSave = isRootShareFolder
          ? _files
              .map((file) => file.path)
              .where((path) => path.trim().isNotEmpty)
              .toList()
          : <String>[uri];

      await ShareLinkService.instance.saveSharedFiles(
        uris: urisToSave,
        destination: destination,
        contextHint: _contextHint,
        shareId: widget.candidate.id,
        password: widget.candidate.password,
        fileName: name,
      );

      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('已转存「$name」到 $destination')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('转存失败：$e')),
      );
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<String?> _pickDestination() {
    return showModalBottomSheet<String>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      builder: (context) {
        return Padding(
          padding: EdgeInsets.only(
            left: 16,
            right: 16,
            bottom: MediaQuery.of(context).viewInsets.bottom + 16,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                '选择转存位置',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w800,
                    ),
              ),
              const SizedBox(height: 12),
              FolderPicker(
                currentPath: '/',
                maxVisibleItems: 7,
                onFolderSelected: (path) => Navigator.of(context).pop(path),
              ),
            ],
          ),
        );
      },
    );
  }

  Future<void> _enterFolder(ShareLinkFile file) async {
    if (!file.isFolder || file.path.isEmpty) return;
    _breadcrumbs.add(_ShareBreadcrumb(title: file.name, uri: file.path));
    await _loadFolder(file.path);
  }

  Future<void> _jumpToBreadcrumb(int index) async {
    if (index < 0 || index >= _breadcrumbs.length) return;
    final crumb = _breadcrumbs[index];
    _breadcrumbs.removeRange(index + 1, _breadcrumbs.length);
    await _loadFolder(crumb.uri);
  }

  Future<bool> _handleBack() async {
    if (_breadcrumbs.length > 1) {
      final target = _breadcrumbs[_breadcrumbs.length - 2];
      _breadcrumbs.removeLast();
      await _loadFolder(target.uri);
      return false;
    }
    return true;
  }

  @override
  Widget build(BuildContext context) {
    return PopScope(
      canPop: _breadcrumbs.length <= 1,
      onPopInvokedWithResult: (didPop, result) async {
        if (!didPop) {
          await _handleBack();
        }
      },
      child: Scaffold(
        appBar: AppBar(
          title: const Text('文件分享'),
          actions: [
            IconButton(
              tooltip: '浏览器打开',
              icon: const Icon(LucideIcons.externalLink),
              onPressed: _openExternal,
            ),
            IconButton(
              tooltip: '刷新',
              icon: const Icon(Icons.refresh),
              onPressed: _refresh,
            ),
          ],
        ),
        body: RefreshIndicator(
          onRefresh: _refresh,
          child: ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const EdgeInsets.fromLTRB(16, 10, 16, 24),
            children: [
              _buildHeader(context),
              const SizedBox(height: 14),
              _buildContent(context),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildHeader(BuildContext context) {
    final theme = Theme.of(context);
    final info = _info;

    return Card(
      elevation: 0,
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.55),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(22)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (_loadingInfo && info == null)
              const LinearProgressIndicator()
            else if (info != null)
              _buildOwnerBlock(context, info)
            else
              _buildLoadingOwnerFallback(context),
            if (info != null) ...[
              const SizedBox(height: 14),
              _buildShareInfoBlock(context, info),
            ],
            if (_error != null) ...[
              const SizedBox(height: 12),
              _ErrorBox(text: '分享信息读取失败：$_error'),
            ],
            if (_shouldShowPasswordInput(info)) ...[
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _passwordController,
                      decoration: const InputDecoration(
                        labelText: '分享密码',
                        isDense: true,
                        border: OutlineInputBorder(),
                      ),
                      onSubmitted: (_) => _loadShare(
                        password: _passwordController.text.trim(),
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  FilledButton(
                    onPressed: _loadingInfo
                        ? null
                        : () => _loadShare(
                              password: _passwordController.text.trim(),
                            ),
                    child: const Text('解锁'),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildLoadingOwnerFallback(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        CircleAvatar(
          radius: 28,
          backgroundColor: theme.colorScheme.primaryContainer,
          child: Icon(Icons.ios_share, color: theme.colorScheme.onPrimaryContainer),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Text(
            '分享链接',
            style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w800),
          ),
        ),
      ],
    );
  }

  Widget _buildOwnerBlock(BuildContext context, ShareLinkInfo info) {
    final theme = Theme.of(context);
    final ownerName = info.ownerName?.trim().isNotEmpty == true
        ? info.ownerName!.trim()
        : '匿名用户';
    final ownerId = info.ownerId ?? '';

    return Row(
      children: [
        _isSameOrigin
            ? UserAvatar(
                userId: ownerId,
                displayName: ownerName,
                radius: 29,
              )
            : CircleAvatar(
                radius: 29,
                backgroundColor: theme.colorScheme.primaryContainer,
                child: Text(
                  '匿',
                  style: TextStyle(
                    color: theme.colorScheme.onPrimaryContainer,
                    fontWeight: FontWeight.w900,
                    fontSize: 20,
                  ),
                ),
              ),
        const SizedBox(width: 13),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                ownerName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w900,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                '向您分享了 ${info.isFolder ? '一个文件夹' : '一个文件'}',
                style: theme.textTheme.bodySmall?.copyWith(color: theme.hintColor),
              ),
            ],
          ),
        ),
        const SizedBox(width: 8),
        _StatusBadge(
          text: info.expired ? '已过期' : '有效',
          color: info.expired ? theme.colorScheme.error : theme.colorScheme.primary,
        ),
      ],
    );
  }

  Widget _buildShareInfoBlock(BuildContext context, ShareLinkInfo info) {
    final theme = Theme.of(context);
    final file = _singleFile;
    final sizeText = file != null && file.isFile && file.size > 0
        ? _ShareFileTile.formatSize(file.size)
        : null;

    return Container(
      padding: const EdgeInsets.all(13),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface.withValues(alpha: 0.72),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Icon(
                info.isFolder ? LucideIcons.folder : LucideIcons.file,
                size: 22,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  info.name,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w900),
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _MetaChip(icon: LucideIcons.eye, text: '${info.visited} 次访问'),
              if (sizeText != null) _MetaChip(icon: Icons.sd_storage_outlined, text: sizeText),
              if (info.createdAt != null)
                _MetaChip(icon: LucideIcons.calendar, text: '${_formatDate(info.createdAt!)} 创建'),
              if (info.expires != null)
                _MetaChip(icon: LucideIcons.clock, text: '${_formatDate(info.expires!)} 过期'),
              if (info.isPrivate) const _MetaChip(icon: LucideIcons.lock, text: '私密分享'),
            ],
          ),
        ],
      ),
    );
  }

  bool _shouldShowPasswordInput(ShareLinkInfo? info) {
    if (info == null) return _error != null;
    return info.isPrivate && !info.unlocked;
  }

  Widget _buildContent(BuildContext context) {
    final info = _info;

    if (_loadingInfo && info == null) {
      return const Padding(
        padding: EdgeInsets.only(top: 80),
        child: Center(child: CircularProgressIndicator()),
      );
    }

    if (info == null) {
      return _EmptyState(
        icon: LucideIcons.link,
        title: '无法打开分享',
        subtitle: '请检查链接是否正确，或输入分享密码后重试。',
        actionText: '浏览器打开',
        onAction: _openExternal,
      );
    }

    if (info.expired) {
      return const _EmptyState(
        icon: LucideIcons.clock,
        title: '分享已过期',
        subtitle: '这个分享链接已经失效。',
      );
    }

    if (info.isPrivate && !info.unlocked) {
      return const _EmptyState(
        icon: LucideIcons.lock,
        title: '需要分享密码',
        subtitle: '输入正确的分享密码后即可查看文件。',
      );
    }

    if (info.isFile) {
      return _buildSingleFileCard(context, info);
    }

    return _buildFolderList(context, info);
  }

  Widget _buildSingleFileCard(BuildContext context, ShareLinkInfo info) {
    final file = _singleFile ?? ShareLinkService.instance.fileFromShareInfo(info);
    final sourceUri = (info.sourceUri?.trim().isNotEmpty == true)
        ? info.sourceUri
        : (file.path.trim().isNotEmpty ? file.path : null);
    final primaryEntity = file.primaryEntity;

    return Card(
      elevation: 0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                const Icon(LucideIcons.file, size: 34),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        info.name,
                        maxLines: 3,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w900,
                            ),
                      ),
                      ...[
                      const SizedBox(height: 4),
                      Text(
                        '${file.size > 0 ? _ShareFileTile.formatSize(file.size) : '分享文件'}${file.updatedAt == null ? '' : ' · ${_formatDate(file.updatedAt!)}'}',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context).hintColor,
                            ),
                      ),
                    ],
                    ],
                  ),
                ),
              ],
            ),
            if (_loadingFiles) ...[
              const SizedBox(height: 14),
              const LinearProgressIndicator(),
            ],
            if (_fileError != null) ...[
              const SizedBox(height: 14),
              _ErrorBox(
                text: '文件详情读取失败，仍可尝试下载或转存：$_fileError',
                actionText: sourceUri == null ? null : '重试',
                onAction: sourceUri == null ? null : () => _loadSingleFile(sourceUri),
              ),
            ],
            const SizedBox(height: 18),
            _buildActionButtons(
              uri: sourceUri,
              name: info.name,
              fileSize: file.size,
              entity: primaryEntity,
              archive: false,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildActionButtons({
    required String? uri,
    required String name,
    required int fileSize,
    String? entity,
    bool archive = false,
  }) {
    return Row(
      children: [
        Expanded(
          child: FilledButton.tonalIcon(
            onPressed: uri == null || uri.isEmpty || _saving || !_isSameOrigin
                ? null
                : () => _saveSharedUri(uri, name: name),
            icon: _saving
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.drive_folder_upload_outlined, size: 18),
            label: const Text('转存'),
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: FilledButton.icon(
            onPressed: uri == null || uri.isEmpty || _openingDownload
                ? null
                : () => _openDownloadUrl(
                      uri,
                      fileName: name,
                      fileSize: fileSize,
                      entity: entity,
                      archive: archive,
                    ),
            icon: _openingDownload
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(LucideIcons.download, size: 18),
            label: const Text('下载'),
          ),
        ),
      ],
    );
  }

  Widget _buildFolderList(BuildContext context, ShareLinkInfo info) {
    final theme = Theme.of(context);
    final sourceUri = _currentUri ?? info.sourceUri;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (_breadcrumbs.isNotEmpty) _buildBreadcrumbs(context),
        Row(
          children: [
            Expanded(
              child: Text(
                '文件列表',
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w900,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        _buildActionButtons(
          uri: sourceUri,
          name: info.name,
          fileSize: 0,
          archive: true,
        ),
        const SizedBox(height: 12),
        if (_loadingFiles)
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 32),
            child: Center(child: CircularProgressIndicator()),
          )
        else if (_fileError != null)
          _ErrorBox(
            text: '文件列表读取失败：$_fileError',
            actionText: '重试',
            onAction: sourceUri == null ? null : () => _loadFolder(sourceUri),
          )
        else if (_files.isEmpty)
          const _EmptyState(
            icon: LucideIcons.folderOpen,
            title: '文件夹为空',
            subtitle: '这个分享目录下没有文件。',
          )
        else
          ..._files.map((file) => _ShareFileTile(
                file: file,
                onTap: file.isFolder ? () => _enterFolder(file) : null,
                onDownload: file.isFile
                    ? () => _openDownloadUrl(
                          file.path,
                          fileName: file.name,
                          fileSize: file.size,
                          entity: file.primaryEntity,
                        )
                    : null,
                onSave: _isSameOrigin ? () => _saveSharedUri(file.path, name: file.name) : null,
              )),
      ],
    );
  }

  Widget _buildBreadcrumbs(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.55),
        borderRadius: BorderRadius.circular(14),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: List.generate(_breadcrumbs.length, (index) {
            final item = _breadcrumbs[index];
            final isLast = index == _breadcrumbs.length - 1;
            return Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                InkWell(
                  onTap: isLast ? null : () => _jumpToBreadcrumb(index),
                  borderRadius: BorderRadius.circular(999),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
                    child: Text(
                      item.title,
                      style: TextStyle(
                        fontWeight: isLast ? FontWeight.w800 : FontWeight.w500,
                        color: isLast ? theme.colorScheme.primary : theme.hintColor,
                      ),
                    ),
                  ),
                ),
                if (!isLast)
                  Icon(Icons.chevron_right, size: 18, color: theme.hintColor),
              ],
            );
          }),
        ),
      ),
    );
  }

  static String _formatDate(DateTime value) {
    final local = value.toLocal();
    return '${local.year}-${local.month.toString().padLeft(2, '0')}-${local.day.toString().padLeft(2, '0')}';
  }
}

class _ShareBreadcrumb {
  final String title;
  final String uri;

  const _ShareBreadcrumb({required this.title, required this.uri});
}

class _ShareFileTile extends StatelessWidget {
  final ShareLinkFile file;
  final VoidCallback? onTap;
  final VoidCallback? onDownload;
  final VoidCallback? onSave;

  const _ShareFileTile({
    required this.file,
    this.onTap,
    this.onDownload,
    this.onSave,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = file.isFolder ? LucideIcons.folder : _iconForFile(file.name);

    return Card(
      elevation: 0,
      margin: const EdgeInsets.only(bottom: 10),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: ListTile(
        onTap: onTap,
        leading: Container(
          width: 42,
          height: 42,
          decoration: BoxDecoration(
            color: file.isFolder
                ? theme.colorScheme.primaryContainer
                : theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(13),
          ),
          child: Icon(
            icon,
            color: file.isFolder
                ? theme.colorScheme.onPrimaryContainer
                : theme.colorScheme.primary,
          ),
        ),
        title: Text(
          file.name,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(fontWeight: FontWeight.w700),
        ),
        subtitle: Padding(
          padding: const EdgeInsets.only(top: 4),
          child: Text(
            file.isFolder
                ? '文件夹'
                : '${formatSize(file.size)}${file.updatedAt == null ? '' : ' · ${_formatDate(file.updatedAt!)}'}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        trailing: file.isFolder
            ? const Icon(Icons.chevron_right)
            : Wrap(
                spacing: 2,
                children: [
                  IconButton(
                    tooltip: '转存',
                    icon: const Icon(Icons.drive_folder_upload_outlined),
                    onPressed: onSave,
                  ),
                  IconButton(
                    tooltip: '下载',
                    icon: const Icon(LucideIcons.download),
                    onPressed: onDownload,
                  ),
                ],
              ),
      ),
    );
  }

  static IconData _iconForFile(String name) {
    final lower = name.toLowerCase();
    if (RegExp(r'\.(png|jpg|jpeg|gif|webp|bmp|heic)$').hasMatch(lower)) {
      return LucideIcons.image;
    }
    if (RegExp(r'\.(mp4|mkv|mov|avi|webm|flv)$').hasMatch(lower)) {
      return LucideIcons.video;
    }
    if (RegExp(r'\.(mp3|wav|flac|aac|ogg|m4a)$').hasMatch(lower)) {
      return LucideIcons.music;
    }
    if (RegExp(r'\.(zip|rar|7z|tar|gz)$').hasMatch(lower)) {
      return LucideIcons.archive;
    }
    if (RegExp(r'\.(pdf|doc|docx|xls|xlsx|ppt|pptx|txt|md)$').hasMatch(lower)) {
      return LucideIcons.fileText;
    }
    return LucideIcons.file;
  }

  static String formatSize(int bytes) {
    if (bytes <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    var size = bytes.toDouble();
    var unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }
    final text = unitIndex == 0 ? size.toStringAsFixed(0) : size.toStringAsFixed(1);
    return '$text ${units[unitIndex]}';
  }

  static String _formatDate(DateTime value) {
    final local = value.toLocal();
    return '${local.year}-${local.month.toString().padLeft(2, '0')}-${local.day.toString().padLeft(2, '0')}';
  }
}

class _MetaChip extends StatelessWidget {
  final IconData icon;
  final String text;

  const _MetaChip({
    required this.icon,
    required this.text,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 13, color: theme.hintColor),
          const SizedBox(width: 5),
          Text(text, style: theme.textTheme.labelSmall),
        ],
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  final String text;
  final Color color;

  const _StatusBadge({required this.text, required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        text,
        style: TextStyle(
          color: color,
          fontWeight: FontWeight.w800,
          fontSize: 12,
        ),
      ),
    );
  }
}

class _ErrorBox extends StatelessWidget {
  final String text;
  final String? actionText;
  final VoidCallback? onAction;

  const _ErrorBox({
    required this.text,
    this.actionText,
    this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            text,
            style: TextStyle(color: theme.colorScheme.onErrorContainer),
          ),
          if (actionText != null && onAction != null) ...[
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerRight,
              child: TextButton(
                onPressed: onAction,
                child: Text(actionText!),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final String? actionText;
  final VoidCallback? onAction;

  const _EmptyState({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.actionText,
    this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 56),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 46, color: theme.hintColor),
          const SizedBox(height: 12),
          Text(
            title,
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w800,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            subtitle,
            textAlign: TextAlign.center,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.hintColor,
            ),
          ),
          if (actionText != null && onAction != null) ...[
            const SizedBox(height: 14),
            OutlinedButton(
              onPressed: onAction,
              child: Text(actionText!),
            ),
          ],
        ],
      ),
    );
  }
}
