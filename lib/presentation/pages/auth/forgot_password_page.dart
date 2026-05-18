import 'package:cloudreve4_flutter/data/models/login_config_model.dart';
import 'package:cloudreve4_flutter/presentation/widgets/desktop_constrained.dart';
import 'package:cloudreve4_flutter/services/captcha_service.dart';
import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import '../../../core/validators/string_validator.dart';
import '../../../services/auth_service.dart';
import '../../../services/server_service.dart';
import '../../widgets/toast_helper.dart';

class ForgotPasswordPage extends StatefulWidget {
  final LoginConfigModel loginConfig;

  const ForgotPasswordPage({super.key, this.loginConfig = const LoginConfigModel()});

  @override
  State<ForgotPasswordPage> createState() => _ForgotPasswordPageState();
}

class _ForgotPasswordPageState extends State<ForgotPasswordPage> {
  final _formKey = GlobalKey<FormState>();
  final _emailController = TextEditingController();
  bool _isLoading = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    if (widget.loginConfig.forgetCaptcha) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final server = ServerService.instance.currentServer;
        if (server != null) {
          CaptchaService.instance.loadCaptcha(server.baseUrl).then((_) {
            if (mounted) setState(() {});
          });
        }
      });
    }
  }

  @override
  void dispose() {
    _emailController.dispose();
    super.dispose();
  }

  Future<void> _sendResetEmail() async {
    if (!_formKey.currentState!.validate()) return;

    final captcha = CaptchaService.instance;
    if (widget.loginConfig.forgetCaptcha && !captcha.isWebCaptchaVerified) {
      ToastHelper.failure('请先完成人机验证');
      return;
    }

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final captchaParams = widget.loginConfig.forgetCaptcha
          ? captcha.getCaptchaParams()
          : <String, String>{};

      await AuthService.instance.sendResetPasswordEmail(
        email: _emailController.text.trim(),
        captcha: captchaParams['captcha'],
        ticket: captchaParams['ticket'],
      );

      if (mounted) {
        ToastHelper.success('重置密码邮件已发送，请查收邮箱');
        Navigator.of(context).pop();
      }
    } catch (e) {
      if (mounted) {
        if (widget.loginConfig.forgetCaptcha) {
          await captcha.refreshCaptcha();
          setState(() {});
        }
        setState(() => _errorMessage = e.toString());
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final captcha = CaptchaService.instance;

    return Scaffold(
      appBar: AppBar(title: const Text('忘记密码')),
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: DesktopConstrained(
              maxContentWidth: 480,
              child: Card(
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Form(
                    key: _formKey,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Text(
                          '请输入您的邮箱地址，我们将向您发送重置密码的邮件。',
                          style: TextStyle(
                            fontSize: 14,
                            color: theme.hintColor,
                          ),
                        ),
                        const SizedBox(height: 24),
                        TextFormField(
                          controller: _emailController,
                          keyboardType: TextInputType.emailAddress,
                          validator: StringValidator.validateEmail,
                          decoration: const InputDecoration(
                            labelText: '邮箱',
                            hintText: '请输入邮箱地址',
                            prefixIcon: Icon(LucideIcons.mail),
                          ),
                        ),
                        if (widget.loginConfig.forgetCaptcha) ...[
                          const SizedBox(height: 16),
                          captcha.buildCaptchaInput(context),
                        ],
                        if (_errorMessage != null) ...[
                          const SizedBox(height: 16),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                            decoration: BoxDecoration(
                              color: theme.colorScheme.errorContainer,
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: Text(
                              _errorMessage!,
                              style: TextStyle(
                                color: theme.colorScheme.onErrorContainer,
                                fontSize: 13,
                              ),
                            ),
                          ),
                        ],
                        const SizedBox(height: 24),
                        FilledButton(
                          onPressed: _isLoading ? null : _sendResetEmail,
                          style: FilledButton.styleFrom(
                            minimumSize: const Size(double.infinity, 48),
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(12),
                            ),
                          ),
                          child: _isLoading
                              ? const SizedBox(
                                  width: 24,
                                  height: 24,
                                  child: CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Text('发送重置邮件'),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
