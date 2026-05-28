use crate::errors::SyncError;

/// Cloudreve 服务端错误码 → 中文描述映射
pub fn server_code_desc(code: i32) -> Option<&'static str> {
    Some(match code {
        // HTTP 语义码
        203 => "部分操作未成功",
        401 => "未登录",
        403 => "无权限访问",
        404 => "资源不存在",
        409 => "资源冲突",
        // 4xxxx 业务码
        40001 => "参数错误",
        40002 => "上传失败",
        40003 => "文件夹创建失败",
        40004 => "对象已存在",
        40005 => "签名已过期",
        40006 => "当前存储策略不允许",
        40007 => "用户组不允许此操作",
        40008 => "需要管理员权限",
        40009 => "主节点未注册",
        40010 => "需要绑定手机",
        40011 => "上传会话已过期",
        40012 => "无效的分片序号",
        40013 => "无效的 Content-Length",
        40014 => "批量源大小超限",
        40016 => "父目录不存在",
        40017 => "用户被封禁",
        40018 => "用户未激活",
        40019 => "功能未启用",
        40020 => "凭据无效",
        40021 => "用户不存在",
        40022 => "两步验证码错误",
        40023 => "登录会话不存在",
        40026 => "验证码错误",
        40027 => "验证码需要刷新",
        40035 => "存储策略不存在",
        40044 => "文件未找到",
        40045 => "列出文件失败",
        40049 => "文件过大",
        40050 => "文件类型不允许",
        40051 => "用户容量不足",
        40052 => "非法对象名",
        40053 => "根目录受保护",
        40054 => "同名文件正在上传",
        40055 => "元数据不匹配",
        40057 => "可用存储策略已变更",
        40071 => "签名无效",
        40073 => "文件锁定冲突",
        40074 => "URI 数量过多",
        40075 => "锁令牌已过期",
        40077 => "实体不存在",
        40078 => "文件在回收站中",
        40079 => "文件数量已达上限",
        40081 => "批量操作未完全完成",
        40082 => "仅所有者可操作",
        // 5xxxx 服务端内部错误
        50001 => "数据库操作失败",
        50002 => "加密失败",
        50004 => "IO 操作失败",
        50006 => "缓存操作失败",
        50007 => "回调失败",
        50010 => "节点离线",
        _ => return None,
    })
}

/// 将服务端业务错误码转为 SyncError
pub fn api_code_to_error(code: i32, msg: &str) -> SyncError {
    let desc = server_code_desc(code);
    let detail = if msg.is_empty() {
        desc.unwrap_or("未知错误").to_string()
    } else if let Some(d) = desc {
        if msg != d {
            format!("{}: {}", d, msg)
        } else {
            msg.to_string()
        }
    } else {
        msg.to_string()
    };

    match code {
        401 => SyncError::Auth(detail),
        40004 => SyncError::ObjectExisted,
        40006 | 40035 | 40057 => SyncError::StoragePolicyDenied(detail),
        40002 | 40011 | 40012 | 40013 | 40054 => SyncError::UploadFailed(detail),
        40044 | 40077 => SyncError::FileNotFound(detail),
        40017 | 40018 | 40007 | 40008 => SyncError::PermissionDenied(detail),
        _ => SyncError::Network(format!("[{}] {}", code, detail)),
    }
}
