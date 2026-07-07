/// 分享模型
class ShareModel {
  final String id;
  final String name;
  final int visited;
  final int? downloaded;
  final int? price;
  final bool unlocked;
  final int sourceType;
  final ShareOwner? owner;
  final DateTime createdAt;
  final DateTime? expires;
  final bool expired;
  final String url;
  final int? size;
  final SharePermissionSetting? permissionSetting;
  final bool? isPrivate;
  final String? password;
  final bool? shareView;
  final String? sourceUri;
  final bool? showReadme;
  final bool? passwordProtected;

  ShareModel({
    required this.id,
    required this.name,
    required this.visited,
    this.downloaded,
    this.price,
    required this.unlocked,
    required this.sourceType,
    this.owner,
    required this.createdAt,
    this.expires,
    required this.expired,
    required this.url,
    this.size,
    this.permissionSetting,
    this.isPrivate,
    this.password,
    this.shareView,
    this.sourceUri,
    this.showReadme,
    this.passwordProtected,
  });

  factory ShareModel.fromJson(Map<String, dynamic> json) {
    final ownerRaw = json['owner'];
    final permissionRaw = json['permission_setting'];
    final createdAtRaw = json['created_at'] ?? json['create_date'] ?? json['createdAt'];
    final expiresRaw = json['expires'] ?? json['expire'] ?? json['expired_at'];
    final sourceTypeRaw = json['source_type'] ?? json['type'];

    return ShareModel(
      id: json['id']?.toString() ?? json['key']?.toString() ?? '',
      name: json['name']?.toString() ?? json['source_name']?.toString() ?? '',
      visited: (json['visited'] as num?)?.toInt() ?? 0,
      downloaded: (json['downloaded'] as num?)?.toInt(),
      price: (json['price'] as num?)?.toInt(),
      unlocked: json['unlocked'] as bool? ?? false,
      sourceType: sourceTypeRaw is num
          ? sourceTypeRaw.toInt()
          : (sourceTypeRaw == 'dir' || sourceTypeRaw == 'folder' ? 1 : 0),
      owner: ownerRaw is Map ? ShareOwner.fromJson(Map<String, dynamic>.from(ownerRaw)) : null,
      createdAt: DateTime.tryParse(createdAtRaw?.toString() ?? '') ?? DateTime.now(),
      expires: DateTime.tryParse(expiresRaw?.toString() ?? ''),
      expired: json['expired'] as bool? ?? false,
      url: json['url']?.toString() ?? '',
      size: (json['size'] as num?)?.toInt(),
      permissionSetting: permissionRaw is Map
          ? SharePermissionSetting.fromJson(Map<String, dynamic>.from(permissionRaw))
          : null,
      isPrivate: json['is_private'] as bool?,
      password: json['password']?.toString(),
      shareView: json['share_view'] as bool?,
      sourceUri: json['source_uri']?.toString() ?? json['uri']?.toString(),
      showReadme: json['show_readme'] as bool?,
      passwordProtected: json['password_protected'] as bool?,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'visited': visited,
      'downloaded': downloaded,
      'price': price,
      'unlocked': unlocked,
      'source_type': sourceType,
      'owner': owner?.toJson(),
      'created_at': createdAt.toIso8601String(),
      'expires': expires?.toIso8601String(),
      'expired': expired,
      'url': url,
      'size': size,
      'permission_setting': permissionSetting?.toJson(),
      'is_private': isPrivate,
      'password': password,
      'share_view': shareView,
      'source_uri': sourceUri,
      'show_readme': showReadme,
      'password_protected': passwordProtected,
    };
  }

  bool get isFolder => sourceType == 1;
  bool get isFile => sourceType == 0;
}

/// 分享所有者信息
class ShareOwner {
  final String id;
  final String? email;
  final String nickname;
  final DateTime createdAt;
  final ShareOwnerGroup? group;
  final String? shareLinksInProfile;

  ShareOwner({
    required this.id,
    this.email,
    required this.nickname,
    required this.createdAt,
    this.group,
    this.shareLinksInProfile,
  });

  factory ShareOwner.fromJson(Map<String, dynamic> json) {
    final groupRaw = json['group'];
    return ShareOwner(
      id: json['id']?.toString() ?? '',
      email: json['email']?.toString(),
      nickname: json['nickname']?.toString() ?? json['user_name']?.toString() ?? '',
      createdAt: DateTime.tryParse(json['created_at']?.toString() ?? '') ?? DateTime.now(),
      group: groupRaw is Map ? ShareOwnerGroup.fromJson(Map<String, dynamic>.from(groupRaw)) : null,
      shareLinksInProfile: json['share_links_in_profile']?.toString(),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'email': email,
      'nickname': nickname,
      'created_at': createdAt.toIso8601String(),
      'group': group?.toJson(),
      'share_links_in_profile': shareLinksInProfile,
    };
  }
}

/// 分享所有者所属组
class ShareOwnerGroup {
  final String id;
  final String name;

  ShareOwnerGroup({required this.id, required this.name});

  factory ShareOwnerGroup.fromJson(Map<String, dynamic> json) {
    return ShareOwnerGroup(
      id: json['id']?.toString() ?? '',
      name: json['name']?.toString() ?? '',
    );
  }

  Map<String, dynamic> toJson() => {'id': id, 'name': name};
}

/// 权限设置
class SharePermissionSetting {
  final String? sameGroup;
  final String? other;
  final String? anonymous;
  final String? everyone;
  final Map<String, String>? groupExplicit;
  final Map<String, String>? userExplicit;

  SharePermissionSetting({
    this.sameGroup,
    this.other,
    this.anonymous,
    this.everyone,
    this.groupExplicit,
    this.userExplicit,
  });

  factory SharePermissionSetting.fromJson(Map<String, dynamic> json) {
    return SharePermissionSetting(
      sameGroup: json['same_group']?.toString(),
      other: json['other']?.toString(),
      anonymous: json['anonymous']?.toString(),
      everyone: json['everyone']?.toString(),
      groupExplicit: _stringMap(json['group_explicit']),
      userExplicit: _stringMap(json['user_explicit']),
    );
  }

  static Map<String, String>? _stringMap(dynamic value) {
    if (value is! Map) return null;
    return value.map((key, val) => MapEntry(key.toString(), val.toString()));
  }

  Map<String, dynamic> toJson() {
    return {
      'same_group': sameGroup,
      'other': other,
      'anonymous': anonymous,
      'everyone': everyone,
      'group_explicit': groupExplicit,
      'user_explicit': userExplicit,
    };
  }
}
