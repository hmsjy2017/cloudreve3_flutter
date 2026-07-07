/// 用户模型
class UserModel {
  final String id;
  final String? email;
  final String nickname;
  final String? avatar;
  final DateTime createdAt;
  final String? preferredTheme;
  final String? language;
  final bool? anonymous;
  final GroupModel? group;
  final List<PinedFileModel>? pined;

  // Token 信息
  final TokenModel? token;

  UserModel({
    required this.id,
    this.email,
    required this.nickname,
    this.avatar,
    required this.createdAt,
    this.preferredTheme,
    this.language,
    this.anonymous,
    this.group,
    this.pined,
    this.token,
  });

  factory UserModel.fromJson(Map<String, dynamic> json) {
    final createdAtRaw = json['created_at'] ?? json['createdAt'];
    final createdAt = createdAtRaw is String
        ? DateTime.tryParse(createdAtRaw) ?? DateTime.now()
        : DateTime.now();
    final groupRaw = json['group'];
    final pinedRaw = json['pined'];
    final tokenRaw = json['token'];
    final anonymousRaw = json['anonymous'];

    return UserModel(
      id: json['id']?.toString() ?? '',
      email: json['email']?.toString(),
      nickname: json['nickname']?.toString() ??
          json['user_name']?.toString() ??
          json['username']?.toString() ??
          json['email']?.toString() ??
          '',
      avatar: json['avatar']?.toString(),
      createdAt: createdAt,
      preferredTheme: json['preferred_theme']?.toString(),
      language: json['language']?.toString(),
      anonymous: anonymousRaw is bool ? anonymousRaw : null,
      group: groupRaw is Map ? GroupModel.fromJson(Map<String, dynamic>.from(groupRaw)) : null,
      pined: pinedRaw is List
          ? pinedRaw
                .whereType<Map>()
                .map((e) => PinedFileModel.fromJson(Map<String, dynamic>.from(e)))
                .toList()
          : null,
      token: tokenRaw is Map ? TokenModel.fromJson(Map<String, dynamic>.from(tokenRaw)) : null,
    );
  }

  UserModel copyWith({
    String? id,
    String? email,
    String? nickname,
    String? avatar,
    DateTime? createdAt,
    String? preferredTheme,
    String? language,
    bool? anonymous,
    GroupModel? group,
    List<PinedFileModel>? pined,
    TokenModel? token,
  }) {
    return UserModel(
      id: id ?? this.id,
      email: email ?? this.email,
      nickname: nickname ?? this.nickname,
      avatar: avatar ?? this.avatar,
      createdAt: createdAt ?? this.createdAt,
      preferredTheme: preferredTheme ?? this.preferredTheme,
      language: language ?? this.language,
      anonymous: anonymous ?? this.anonymous,
      group: group ?? this.group,
      pined: pined ?? this.pined,
      token: token ?? this.token,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'email': email,
      'nickname': nickname,
      'avatar': avatar,
      'created_at': createdAt.toIso8601String(),
      'preferred_theme': preferredTheme,
      'language': language,
      'anonymous': anonymous,
      'group': group?.toJson(),
      'pined': pined?.map((e) => e.toJson()).toList(),
      'token': token?.toJson(),
    };
  }
}

/// 用户组模型
class GroupModel {
  final String id;
  final String name;
  final String? permission;
  final int? directLinkBatchSize;
  final int? trashRetention;

  GroupModel({
    required this.id,
    required this.name,
    this.permission,
    this.directLinkBatchSize,
    this.trashRetention,
  });

  factory GroupModel.fromJson(Map<String, dynamic> json) {
    return GroupModel(
      id: json['id']?.toString() ?? '',
      name: json['name']?.toString() ?? '',
      permission: json['permission']?.toString(),
      directLinkBatchSize: (json['direct_link_batch_size'] as num?)?.toInt(),
      trashRetention: (json['trash_retention'] as num?)?.toInt(),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'permission': permission,
      'direct_link_batch_size': directLinkBatchSize,
      'trash_retention': trashRetention,
    };
  }
}

/// 固定文件模型
class PinedFileModel {
  final String uri;
  final String? name;

  PinedFileModel({required this.uri, this.name});

  factory PinedFileModel.fromJson(Map<String, dynamic> json) {
    return PinedFileModel(
      uri: json['uri']?.toString() ?? '',
      name: json['name']?.toString(),
    );
  }

  Map<String, dynamic> toJson() {
    return {'uri': uri, 'name': name};
  }
}

/// Token模型
class TokenModel {
  final String accessToken;
  final String refreshToken;
  final DateTime accessExpires;
  final DateTime refreshExpires;

  TokenModel({
    required this.accessToken,
    required this.refreshToken,
    required this.accessExpires,
    required this.refreshExpires,
  });

  factory TokenModel.fromJson(Map<String, dynamic> json) {
    // 支持两种格式：直接是 token 数据或嵌套在 data 中
    Map<String, dynamic> data;

    if (json['data'] != null) {
      data = json['data'] as Map<String, dynamic>;
    } else {
      data = json;
    }

    return TokenModel(
      accessToken: data['access_token']?.toString() ?? '',
      refreshToken: data['refresh_token']?.toString() ?? '',
      accessExpires: DateTime.tryParse(data['access_expires']?.toString() ?? '') ?? DateTime.now(),
      refreshExpires: DateTime.tryParse(data['refresh_expires']?.toString() ?? '') ?? DateTime.now(),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'access_token': accessToken,
      'refresh_token': refreshToken,
      'access_expires': accessExpires.toIso8601String(),
      'refresh_expires': refreshExpires.toIso8601String(),
    };
  }

  /// 检查 access token 是否过期
  bool get isAccessTokenExpired => DateTime.now().isAfter(accessExpires);

  /// 检查 refresh token 是否过期
  bool get isRefreshTokenExpired => DateTime.now().isAfter(refreshExpires);
}

/// 用户容量模型
class CapacityModel {
  final int total;
  final int used;
  double get usagePercentage => total > 0 ? (used / total) * 100 : 0;

  int get remaining => total - used;

  CapacityModel({required this.total, required this.used});

  factory CapacityModel.fromJson(Map<String, dynamic> json) {
    return CapacityModel(
      total: (json['total'] as num?)?.toInt() ?? 0,
      used: (json['used'] as num?)?.toInt() ?? 0,
    );
  }

  Map<String, dynamic> toJson() {
    return {'total': total, 'used': used};
  }
}
