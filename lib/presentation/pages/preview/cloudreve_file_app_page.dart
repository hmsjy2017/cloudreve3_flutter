import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:webview_flutter/webview_flutter.dart' as mobile;

import '../../../data/models/file_model.dart';
import '../../../services/api_service.dart';

bool get _isDesktop => !kIsWeb && (Platform.isWindows || Platform.isLinux);

/// 完全交给 Cloudreve 官方 Web 前端处理文件打开。
///
/// 这里不再自己读取 file_viewers，也不再自己创建 viewerSession。
/// Cloudreve 前端本身会根据 `/home?path=...&open=...` 打开对应文件，
/// 并使用它自己的文件应用、WOPI、Markdown、表格、压缩包、EPUB 等逻辑。
class CloudreveFileAppPage extends StatefulWidget {
  final FileModel file;
  final String? preferredAction;

  const CloudreveFileAppPage({
    super.key,
    required this.file,
    this.preferredAction,
  });

  @override
  State<CloudreveFileAppPage> createState() => _CloudreveFileAppPageState();
}

class _CloudreveFileAppPageState extends State<CloudreveFileAppPage> {
  mobile.WebViewController? _mobileController;
  InAppWebViewController? _desktopController;
  Key _desktopKey = UniqueKey();

  bool _isPreparing = true;
  bool _sessionInjected = false;
  int _progress = 0;
  String? _error;
  late Uri _targetUri;
  late Uri _originUri;
  String? _sessionStateJson;

  @override
  void initState() {
    super.initState();

    if (!_isDesktop) {
      _mobileController = mobile.WebViewController()
        ..setJavaScriptMode(mobile.JavaScriptMode.unrestricted)
        ..setBackgroundColor(Colors.transparent)
        ..setNavigationDelegate(
          mobile.NavigationDelegate(
            onProgress: (progress) {
              if (!mounted) return;
              setState(() => _progress = progress);
            },
            onPageFinished: (_) => _injectSessionAndOpenIfNeeded(),
            onWebResourceError: (error) {
              if (!mounted) return;
              if (error.isForMainFrame == true) {
                setState(() {
                  _error = '${error.errorCode}: ${error.description}';
                });
              }
            },
          ),
        );
    }

    _prepareAndOpen();
  }

  Future<void> _prepareAndOpen() async {
    setState(() {
      _isPreparing = true;
      _error = null;
      _progress = 0;
      _sessionInjected = false;
    });

    try {
      _originUri = _buildOriginUri();
      _targetUri = _buildCloudreveHomeUri();

      _sessionStateJson = await _buildCloudreveFrontendSessionJson();

      if (!_isDesktop && _mobileController != null) {
        await _mobileController!.loadRequest(_originUri);
      } else if (_isDesktop) {
        setState(() => _desktopKey = UniqueKey());
      }

      if (!mounted) return;
      setState(() => _isPreparing = false);
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _isPreparing = false;
      });
    }
  }

  Uri _buildOriginUri() {
    final base = Uri.parse(ApiService.instance.dio.options.baseUrl);
    return Uri(
      scheme: base.scheme,
      host: base.host,
      port: base.hasPort ? base.port : null,
      path: '/',
    );
  }

  Uri _buildCloudreveHomeUri() {
    final parent = _parentUri(widget.file.relativePath);
    final openTarget = widget.file.id.isNotEmpty ? widget.file.id : widget.file.relativePath;

    return Uri(
      scheme: _originUri.scheme,
      host: _originUri.host,
      port: _originUri.hasPort ? _originUri.port : null,
      path: '/home',
      queryParameters: {
        'path': parent,
        'open': openTarget,
        'size': widget.file.size.toString(),
      },
    );
  }

  String _parentUri(String uri) {
    final normalized = uri.endsWith('/') ? uri.substring(0, uri.length - 1) : uri;
    final index = normalized.lastIndexOf('/');
    if (index <= 'cloudreve://my'.length) {
      return 'cloudreve://my';
    }
    return normalized.substring(0, index);
  }

  Future<String?> _buildCloudreveFrontendSessionJson() async {
    final tokenGetter = ApiService.instance.getTokenCallback;
    final token = tokenGetter == null ? null : await tokenGetter();
    if (token == null || token.isEmpty) {
      return null;
    }

    Map<String, dynamic>? user;
    try {
      final response = await ApiService.instance.get<Map<String, dynamic>>('/user/me');
      user = Map<String, dynamic>.from(response);
    } catch (_) {
      user = null;
    }

    final userId = user?['id']?.toString() ?? user?['uid']?.toString() ?? 'app';

    final now = DateTime.now().toUtc();
    final accessExpires = now.add(const Duration(hours: 2)).toIso8601String();
    final refreshExpires = now.add(const Duration(hours: 2)).toIso8601String();

    final sessionState = {
      'current': userId,
      'sessions': {
        userId: {
          'user': user ?? {'id': userId},
          'token': {
            'access_token': token,
            // App 侧暂时没有把 refresh token 暴露给这里。
            // 给 Cloudreve 前端一个短期可用 session；过期后 WebView 内刷新会要求重新登录。
            'refresh_token': '',
            'access_expires': accessExpires,
            'refresh_expires': refreshExpires,
          },
          'settings': {},
        },
      },
      'anonymousSettings': {},
    };

    return jsonEncode(sessionState);
  }

  Future<void> _injectSessionAndOpenIfNeeded() async {
    if (_sessionInjected) return;
    _sessionInjected = true;

    final sessionJson = _sessionStateJson;
    final target = _targetUri.toString();

    try {
      if (sessionJson != null) {
        final script = '''
          try {
            localStorage.setItem('cloudreve_session', ${jsonEncode(sessionJson)});
          } catch (e) {}
          window.location.replace(${jsonEncode(target)});
        ''';
        if (!_isDesktop && _mobileController != null) {
          await _mobileController!.runJavaScript(script);
        } else if (_isDesktop && _desktopController != null) {
          await _desktopController!.evaluateJavascript(source: script);
        }
      } else {
        if (!_isDesktop && _mobileController != null) {
          await _mobileController!.loadRequest(_targetUri);
        } else if (_isDesktop && _desktopController != null) {
          await _desktopController!.loadUrl(
            urlRequest: URLRequest(url: WebUri(_targetUri.toString())),
          );
        }
      }
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = e.toString());
    }
  }

  Future<void> _openTargetAgain() async {
    _sessionInjected = true;
    if (!_isDesktop && _mobileController != null) {
      await _mobileController!.loadRequest(_targetUri);
    } else if (_isDesktop && _desktopController != null) {
      await _desktopController!.loadUrl(
        urlRequest: URLRequest(url: WebUri(_targetUri.toString())),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(
          widget.file.name,
          overflow: TextOverflow.ellipsis,
        ),
        actions: [
          IconButton(
            tooltip: '重新打开',
            icon: const Icon(Icons.open_in_browser),
            onPressed: _openTargetAgain,
          ),
          IconButton(
            tooltip: '刷新',
            icon: const Icon(Icons.refresh),
            onPressed: () {
              if (!_isDesktop && _mobileController != null) {
                _mobileController!.reload();
              } else if (_isDesktop && _desktopController != null) {
                _desktopController!.reload();
              }
            },
          ),
        ],
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(3),
          child: _progress > 0 && _progress < 100
              ? LinearProgressIndicator(value: _progress / 100)
              : const SizedBox(height: 3),
        ),
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_isPreparing) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.error_outline, size: 56, color: Colors.red),
              const SizedBox(height: 16),
              Text(_error!, textAlign: TextAlign.center),
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: _prepareAndOpen,
                icon: const Icon(Icons.refresh),
                label: const Text('重试'),
              ),
            ],
          ),
        ),
      );
    }

    return _isDesktop ? _buildDesktopWebView() : mobile.WebViewWidget(controller: _mobileController!);
  }

  Widget _buildDesktopWebView() {
    return InAppWebView(
      key: _desktopKey,
      initialSettings: InAppWebViewSettings(
        javaScriptEnabled: true,
        domStorageEnabled: true,
        isInspectable: true,
        cacheMode: CacheMode.LOAD_DEFAULT,
        supportMultipleWindows: true,
        transparentBackground: true,
        supportZoom: true,
        useHybridComposition: true,
      ),
      onWebViewCreated: (controller) {
        _desktopController = controller;
        controller.loadUrl(
          urlRequest: URLRequest(url: WebUri(_originUri.toString())),
        );
      },
      onLoadStop: (controller, url) {
        _desktopController = controller;
        _injectSessionAndOpenIfNeeded();
      },
      onProgressChanged: (controller, progress) {
        if (mounted) setState(() => _progress = progress);
      },
      onReceivedError: (controller, request, error) {
        if (mounted) {
          setState(() {
            _error = '${error.type}: ${error.description}'.trim();
          });
        }
      },
    );
  }

  @override
  void dispose() {
    _desktopController?.dispose();
    super.dispose();
  }
}
