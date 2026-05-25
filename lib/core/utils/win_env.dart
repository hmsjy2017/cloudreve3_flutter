import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

/// 通过 Win32 SetEnvironmentVariableW 设置/清除进程环境变量
/// 仅 Windows 有效，其他平台直接返回 false
bool winSetEnvVar(String name, String? value) {
  if (!Platform.isWindows) return false;

  final dylib = DynamicLibrary.open('kernel32.dll');
  final fn = dylib.lookupFunction<
      Int32 Function(Pointer<Utf16>, Pointer<Utf16>),
      int Function(Pointer<Utf16>, Pointer<Utf16>)>(
    'SetEnvironmentVariableW',
  );

  final namePtr = name.toNativeUtf16();
  final valuePtr = value != null ? value.toNativeUtf16() : nullptr;
  try {
    return fn(namePtr, valuePtr) != 0;
  } finally {
    calloc.free(namePtr);
    if (value != null) calloc.free(valuePtr);
  }
}
