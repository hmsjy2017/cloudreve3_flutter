import '../../src/rust/api/ffi_types.dart' as ffi;

class SyncConfigModel {
  final String baseUrl;
  final String accessToken;
  final String localRoot;
  final String remoteRoot;
  final String syncMode;
  final String conflictStrategy;
  final int maxConcurrentTransfers;
  final int bandwidthLimitKbps;
  final List<String> excludedPaths;
  final List<String> selectiveDirs;

  const SyncConfigModel({
    required this.baseUrl,
    required this.accessToken,
    required this.localRoot,
    this.remoteRoot = 'cloudreve://my',
    this.syncMode = 'full',
    this.conflictStrategy = 'keep_both',
    this.maxConcurrentTransfers = 3,
    this.bandwidthLimitKbps = 0,
    this.excludedPaths = const [],
    this.selectiveDirs = const [],
  });

  SyncConfigModel copyWith({
    String? baseUrl,
    String? accessToken,
    String? localRoot,
    String? remoteRoot,
    String? syncMode,
    String? conflictStrategy,
    int? maxConcurrentTransfers,
    int? bandwidthLimitKbps,
    List<String>? excludedPaths,
    List<String>? selectiveDirs,
  }) {
    return SyncConfigModel(
      baseUrl: baseUrl ?? this.baseUrl,
      accessToken: accessToken ?? this.accessToken,
      localRoot: localRoot ?? this.localRoot,
      remoteRoot: remoteRoot ?? this.remoteRoot,
      syncMode: syncMode ?? this.syncMode,
      conflictStrategy: conflictStrategy ?? this.conflictStrategy,
      maxConcurrentTransfers:
          maxConcurrentTransfers ?? this.maxConcurrentTransfers,
      bandwidthLimitKbps: bandwidthLimitKbps ?? this.bandwidthLimitKbps,
      excludedPaths: excludedPaths ?? this.excludedPaths,
      selectiveDirs: selectiveDirs ?? this.selectiveDirs,
    );
  }

  ffi.SyncConfigFfi toFfi() {
    return ffi.SyncConfigFfi(
      baseUrl: baseUrl,
      accessToken: accessToken,
      localRoot: localRoot,
      remoteRoot: remoteRoot,
      syncMode: syncMode,
      conflictStrategy: conflictStrategy,
      maxConcurrentTransfers: maxConcurrentTransfers,
      bandwidthLimitKbps: BigInt.from(bandwidthLimitKbps),
      excludedPaths: excludedPaths,
      selectiveDirs: selectiveDirs,
    );
  }

  static SyncConfigModel fromFfi(ffi.SyncConfigFfi ffi) {
    return SyncConfigModel(
      baseUrl: ffi.baseUrl,
      accessToken: ffi.accessToken,
      localRoot: ffi.localRoot,
      remoteRoot: ffi.remoteRoot,
      syncMode: ffi.syncMode,
      conflictStrategy: ffi.conflictStrategy,
      maxConcurrentTransfers: ffi.maxConcurrentTransfers,
      bandwidthLimitKbps: ffi.bandwidthLimitKbps.toInt(),
      excludedPaths: ffi.excludedPaths,
      selectiveDirs: ffi.selectiveDirs,
    );
  }
}
