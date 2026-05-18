/// 登录配置模型，来自 GET /site/config/login
class LoginConfigModel {
  final bool loginCaptcha;
  final bool regCaptcha;
  final bool forgetCaptcha;
  final bool registerEnabled;
  final String? tosUrl;
  final String? privacyPolicyUrl;

  const LoginConfigModel({
    this.loginCaptcha = false,
    this.regCaptcha = false,
    this.forgetCaptcha = false,
    this.registerEnabled = true,
    this.tosUrl,
    this.privacyPolicyUrl,
  });

  factory LoginConfigModel.fromJson(Map<String, dynamic> json) {
    return LoginConfigModel(
      loginCaptcha: json['login_captcha'] as bool? ?? false,
      regCaptcha: json['reg_captcha'] as bool? ?? false,
      forgetCaptcha: json['forget_captcha'] as bool? ?? false,
      registerEnabled: json['register_enabled'] as bool? ?? true,
      tosUrl: json['tos_url'] as String?,
      privacyPolicyUrl: json['privacy_policy_url'] as String?,
    );
  }
}
