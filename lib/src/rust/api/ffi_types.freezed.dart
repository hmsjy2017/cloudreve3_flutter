// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'ffi_types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$SyncErrorFfi {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncErrorFfi()';
}


}

/// @nodoc
class $SyncErrorFfiCopyWith<$Res>  {
$SyncErrorFfiCopyWith(SyncErrorFfi _, $Res Function(SyncErrorFfi) __);
}


/// Adds pattern-matching-related methods to [SyncErrorFfi].
extension SyncErrorFfiPatterns on SyncErrorFfi {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SyncErrorFfi_NotInitialized value)?  notInitialized,TResult Function( SyncErrorFfi_NetworkError value)?  networkError,TResult Function( SyncErrorFfi_DiskFull value)?  diskFull,TResult Function( SyncErrorFfi_AuthError value)?  authError,TResult Function( SyncErrorFfi_ConflictError value)?  conflictError,TResult Function( SyncErrorFfi_InternalError value)?  internalError,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SyncErrorFfi_NotInitialized() when notInitialized != null:
return notInitialized(_that);case SyncErrorFfi_NetworkError() when networkError != null:
return networkError(_that);case SyncErrorFfi_DiskFull() when diskFull != null:
return diskFull(_that);case SyncErrorFfi_AuthError() when authError != null:
return authError(_that);case SyncErrorFfi_ConflictError() when conflictError != null:
return conflictError(_that);case SyncErrorFfi_InternalError() when internalError != null:
return internalError(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SyncErrorFfi_NotInitialized value)  notInitialized,required TResult Function( SyncErrorFfi_NetworkError value)  networkError,required TResult Function( SyncErrorFfi_DiskFull value)  diskFull,required TResult Function( SyncErrorFfi_AuthError value)  authError,required TResult Function( SyncErrorFfi_ConflictError value)  conflictError,required TResult Function( SyncErrorFfi_InternalError value)  internalError,}){
final _that = this;
switch (_that) {
case SyncErrorFfi_NotInitialized():
return notInitialized(_that);case SyncErrorFfi_NetworkError():
return networkError(_that);case SyncErrorFfi_DiskFull():
return diskFull(_that);case SyncErrorFfi_AuthError():
return authError(_that);case SyncErrorFfi_ConflictError():
return conflictError(_that);case SyncErrorFfi_InternalError():
return internalError(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SyncErrorFfi_NotInitialized value)?  notInitialized,TResult? Function( SyncErrorFfi_NetworkError value)?  networkError,TResult? Function( SyncErrorFfi_DiskFull value)?  diskFull,TResult? Function( SyncErrorFfi_AuthError value)?  authError,TResult? Function( SyncErrorFfi_ConflictError value)?  conflictError,TResult? Function( SyncErrorFfi_InternalError value)?  internalError,}){
final _that = this;
switch (_that) {
case SyncErrorFfi_NotInitialized() when notInitialized != null:
return notInitialized(_that);case SyncErrorFfi_NetworkError() when networkError != null:
return networkError(_that);case SyncErrorFfi_DiskFull() when diskFull != null:
return diskFull(_that);case SyncErrorFfi_AuthError() when authError != null:
return authError(_that);case SyncErrorFfi_ConflictError() when conflictError != null:
return conflictError(_that);case SyncErrorFfi_InternalError() when internalError != null:
return internalError(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  notInitialized,TResult Function( String message)?  networkError,TResult Function( BigInt needed,  BigInt available)?  diskFull,TResult Function( String message)?  authError,TResult Function( int count)?  conflictError,TResult Function( String message)?  internalError,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SyncErrorFfi_NotInitialized() when notInitialized != null:
return notInitialized();case SyncErrorFfi_NetworkError() when networkError != null:
return networkError(_that.message);case SyncErrorFfi_DiskFull() when diskFull != null:
return diskFull(_that.needed,_that.available);case SyncErrorFfi_AuthError() when authError != null:
return authError(_that.message);case SyncErrorFfi_ConflictError() when conflictError != null:
return conflictError(_that.count);case SyncErrorFfi_InternalError() when internalError != null:
return internalError(_that.message);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  notInitialized,required TResult Function( String message)  networkError,required TResult Function( BigInt needed,  BigInt available)  diskFull,required TResult Function( String message)  authError,required TResult Function( int count)  conflictError,required TResult Function( String message)  internalError,}) {final _that = this;
switch (_that) {
case SyncErrorFfi_NotInitialized():
return notInitialized();case SyncErrorFfi_NetworkError():
return networkError(_that.message);case SyncErrorFfi_DiskFull():
return diskFull(_that.needed,_that.available);case SyncErrorFfi_AuthError():
return authError(_that.message);case SyncErrorFfi_ConflictError():
return conflictError(_that.count);case SyncErrorFfi_InternalError():
return internalError(_that.message);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  notInitialized,TResult? Function( String message)?  networkError,TResult? Function( BigInt needed,  BigInt available)?  diskFull,TResult? Function( String message)?  authError,TResult? Function( int count)?  conflictError,TResult? Function( String message)?  internalError,}) {final _that = this;
switch (_that) {
case SyncErrorFfi_NotInitialized() when notInitialized != null:
return notInitialized();case SyncErrorFfi_NetworkError() when networkError != null:
return networkError(_that.message);case SyncErrorFfi_DiskFull() when diskFull != null:
return diskFull(_that.needed,_that.available);case SyncErrorFfi_AuthError() when authError != null:
return authError(_that.message);case SyncErrorFfi_ConflictError() when conflictError != null:
return conflictError(_that.count);case SyncErrorFfi_InternalError() when internalError != null:
return internalError(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class SyncErrorFfi_NotInitialized extends SyncErrorFfi {
  const SyncErrorFfi_NotInitialized(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi_NotInitialized);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncErrorFfi.notInitialized()';
}


}




/// @nodoc


class SyncErrorFfi_NetworkError extends SyncErrorFfi {
  const SyncErrorFfi_NetworkError({required this.message}): super._();
  

 final  String message;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncErrorFfi_NetworkErrorCopyWith<SyncErrorFfi_NetworkError> get copyWith => _$SyncErrorFfi_NetworkErrorCopyWithImpl<SyncErrorFfi_NetworkError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi_NetworkError&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'SyncErrorFfi.networkError(message: $message)';
}


}

/// @nodoc
abstract mixin class $SyncErrorFfi_NetworkErrorCopyWith<$Res> implements $SyncErrorFfiCopyWith<$Res> {
  factory $SyncErrorFfi_NetworkErrorCopyWith(SyncErrorFfi_NetworkError value, $Res Function(SyncErrorFfi_NetworkError) _then) = _$SyncErrorFfi_NetworkErrorCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$SyncErrorFfi_NetworkErrorCopyWithImpl<$Res>
    implements $SyncErrorFfi_NetworkErrorCopyWith<$Res> {
  _$SyncErrorFfi_NetworkErrorCopyWithImpl(this._self, this._then);

  final SyncErrorFfi_NetworkError _self;
  final $Res Function(SyncErrorFfi_NetworkError) _then;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(SyncErrorFfi_NetworkError(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncErrorFfi_DiskFull extends SyncErrorFfi {
  const SyncErrorFfi_DiskFull({required this.needed, required this.available}): super._();
  

 final  BigInt needed;
 final  BigInt available;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncErrorFfi_DiskFullCopyWith<SyncErrorFfi_DiskFull> get copyWith => _$SyncErrorFfi_DiskFullCopyWithImpl<SyncErrorFfi_DiskFull>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi_DiskFull&&(identical(other.needed, needed) || other.needed == needed)&&(identical(other.available, available) || other.available == available));
}


@override
int get hashCode => Object.hash(runtimeType,needed,available);

@override
String toString() {
  return 'SyncErrorFfi.diskFull(needed: $needed, available: $available)';
}


}

/// @nodoc
abstract mixin class $SyncErrorFfi_DiskFullCopyWith<$Res> implements $SyncErrorFfiCopyWith<$Res> {
  factory $SyncErrorFfi_DiskFullCopyWith(SyncErrorFfi_DiskFull value, $Res Function(SyncErrorFfi_DiskFull) _then) = _$SyncErrorFfi_DiskFullCopyWithImpl;
@useResult
$Res call({
 BigInt needed, BigInt available
});




}
/// @nodoc
class _$SyncErrorFfi_DiskFullCopyWithImpl<$Res>
    implements $SyncErrorFfi_DiskFullCopyWith<$Res> {
  _$SyncErrorFfi_DiskFullCopyWithImpl(this._self, this._then);

  final SyncErrorFfi_DiskFull _self;
  final $Res Function(SyncErrorFfi_DiskFull) _then;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? needed = null,Object? available = null,}) {
  return _then(SyncErrorFfi_DiskFull(
needed: null == needed ? _self.needed : needed // ignore: cast_nullable_to_non_nullable
as BigInt,available: null == available ? _self.available : available // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class SyncErrorFfi_AuthError extends SyncErrorFfi {
  const SyncErrorFfi_AuthError({required this.message}): super._();
  

 final  String message;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncErrorFfi_AuthErrorCopyWith<SyncErrorFfi_AuthError> get copyWith => _$SyncErrorFfi_AuthErrorCopyWithImpl<SyncErrorFfi_AuthError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi_AuthError&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'SyncErrorFfi.authError(message: $message)';
}


}

/// @nodoc
abstract mixin class $SyncErrorFfi_AuthErrorCopyWith<$Res> implements $SyncErrorFfiCopyWith<$Res> {
  factory $SyncErrorFfi_AuthErrorCopyWith(SyncErrorFfi_AuthError value, $Res Function(SyncErrorFfi_AuthError) _then) = _$SyncErrorFfi_AuthErrorCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$SyncErrorFfi_AuthErrorCopyWithImpl<$Res>
    implements $SyncErrorFfi_AuthErrorCopyWith<$Res> {
  _$SyncErrorFfi_AuthErrorCopyWithImpl(this._self, this._then);

  final SyncErrorFfi_AuthError _self;
  final $Res Function(SyncErrorFfi_AuthError) _then;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(SyncErrorFfi_AuthError(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncErrorFfi_ConflictError extends SyncErrorFfi {
  const SyncErrorFfi_ConflictError({required this.count}): super._();
  

 final  int count;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncErrorFfi_ConflictErrorCopyWith<SyncErrorFfi_ConflictError> get copyWith => _$SyncErrorFfi_ConflictErrorCopyWithImpl<SyncErrorFfi_ConflictError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi_ConflictError&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode => Object.hash(runtimeType,count);

@override
String toString() {
  return 'SyncErrorFfi.conflictError(count: $count)';
}


}

/// @nodoc
abstract mixin class $SyncErrorFfi_ConflictErrorCopyWith<$Res> implements $SyncErrorFfiCopyWith<$Res> {
  factory $SyncErrorFfi_ConflictErrorCopyWith(SyncErrorFfi_ConflictError value, $Res Function(SyncErrorFfi_ConflictError) _then) = _$SyncErrorFfi_ConflictErrorCopyWithImpl;
@useResult
$Res call({
 int count
});




}
/// @nodoc
class _$SyncErrorFfi_ConflictErrorCopyWithImpl<$Res>
    implements $SyncErrorFfi_ConflictErrorCopyWith<$Res> {
  _$SyncErrorFfi_ConflictErrorCopyWithImpl(this._self, this._then);

  final SyncErrorFfi_ConflictError _self;
  final $Res Function(SyncErrorFfi_ConflictError) _then;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? count = null,}) {
  return _then(SyncErrorFfi_ConflictError(
count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SyncErrorFfi_InternalError extends SyncErrorFfi {
  const SyncErrorFfi_InternalError({required this.message}): super._();
  

 final  String message;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncErrorFfi_InternalErrorCopyWith<SyncErrorFfi_InternalError> get copyWith => _$SyncErrorFfi_InternalErrorCopyWithImpl<SyncErrorFfi_InternalError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncErrorFfi_InternalError&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'SyncErrorFfi.internalError(message: $message)';
}


}

/// @nodoc
abstract mixin class $SyncErrorFfi_InternalErrorCopyWith<$Res> implements $SyncErrorFfiCopyWith<$Res> {
  factory $SyncErrorFfi_InternalErrorCopyWith(SyncErrorFfi_InternalError value, $Res Function(SyncErrorFfi_InternalError) _then) = _$SyncErrorFfi_InternalErrorCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$SyncErrorFfi_InternalErrorCopyWithImpl<$Res>
    implements $SyncErrorFfi_InternalErrorCopyWith<$Res> {
  _$SyncErrorFfi_InternalErrorCopyWithImpl(this._self, this._then);

  final SyncErrorFfi_InternalError _self;
  final $Res Function(SyncErrorFfi_InternalError) _then;

/// Create a copy of SyncErrorFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(SyncErrorFfi_InternalError(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$SyncEventFfi {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncEventFfi()';
}


}

/// @nodoc
class $SyncEventFfiCopyWith<$Res>  {
$SyncEventFfiCopyWith(SyncEventFfi _, $Res Function(SyncEventFfi) __);
}


/// Adds pattern-matching-related methods to [SyncEventFfi].
extension SyncEventFfiPatterns on SyncEventFfi {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SyncEventFfi_StateChanged value)?  stateChanged,TResult Function( SyncEventFfi_Progress value)?  progress,TResult Function( SyncEventFfi_FileUploaded value)?  fileUploaded,TResult Function( SyncEventFfi_FileDownloaded value)?  fileDownloaded,TResult Function( SyncEventFfi_ConflictDetected value)?  conflictDetected,TResult Function( SyncEventFfi_Error value)?  error,TResult Function( SyncEventFfi_TokenExpired value)?  tokenExpired,TResult Function( SyncEventFfi_DiskSpaceWarning value)?  diskSpaceWarning,TResult Function( SyncEventFfi_InitialSyncComplete value)?  initialSyncComplete,TResult Function( SyncEventFfi_WorkerStarted value)?  workerStarted,TResult Function( SyncEventFfi_WorkerCompleted value)?  workerCompleted,TResult Function( SyncEventFfi_WorkerFailed value)?  workerFailed,TResult Function( SyncEventFfi_TaskItemUpdated value)?  taskItemUpdated,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SyncEventFfi_StateChanged() when stateChanged != null:
return stateChanged(_that);case SyncEventFfi_Progress() when progress != null:
return progress(_that);case SyncEventFfi_FileUploaded() when fileUploaded != null:
return fileUploaded(_that);case SyncEventFfi_FileDownloaded() when fileDownloaded != null:
return fileDownloaded(_that);case SyncEventFfi_ConflictDetected() when conflictDetected != null:
return conflictDetected(_that);case SyncEventFfi_Error() when error != null:
return error(_that);case SyncEventFfi_TokenExpired() when tokenExpired != null:
return tokenExpired(_that);case SyncEventFfi_DiskSpaceWarning() when diskSpaceWarning != null:
return diskSpaceWarning(_that);case SyncEventFfi_InitialSyncComplete() when initialSyncComplete != null:
return initialSyncComplete(_that);case SyncEventFfi_WorkerStarted() when workerStarted != null:
return workerStarted(_that);case SyncEventFfi_WorkerCompleted() when workerCompleted != null:
return workerCompleted(_that);case SyncEventFfi_WorkerFailed() when workerFailed != null:
return workerFailed(_that);case SyncEventFfi_TaskItemUpdated() when taskItemUpdated != null:
return taskItemUpdated(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SyncEventFfi_StateChanged value)  stateChanged,required TResult Function( SyncEventFfi_Progress value)  progress,required TResult Function( SyncEventFfi_FileUploaded value)  fileUploaded,required TResult Function( SyncEventFfi_FileDownloaded value)  fileDownloaded,required TResult Function( SyncEventFfi_ConflictDetected value)  conflictDetected,required TResult Function( SyncEventFfi_Error value)  error,required TResult Function( SyncEventFfi_TokenExpired value)  tokenExpired,required TResult Function( SyncEventFfi_DiskSpaceWarning value)  diskSpaceWarning,required TResult Function( SyncEventFfi_InitialSyncComplete value)  initialSyncComplete,required TResult Function( SyncEventFfi_WorkerStarted value)  workerStarted,required TResult Function( SyncEventFfi_WorkerCompleted value)  workerCompleted,required TResult Function( SyncEventFfi_WorkerFailed value)  workerFailed,required TResult Function( SyncEventFfi_TaskItemUpdated value)  taskItemUpdated,}){
final _that = this;
switch (_that) {
case SyncEventFfi_StateChanged():
return stateChanged(_that);case SyncEventFfi_Progress():
return progress(_that);case SyncEventFfi_FileUploaded():
return fileUploaded(_that);case SyncEventFfi_FileDownloaded():
return fileDownloaded(_that);case SyncEventFfi_ConflictDetected():
return conflictDetected(_that);case SyncEventFfi_Error():
return error(_that);case SyncEventFfi_TokenExpired():
return tokenExpired(_that);case SyncEventFfi_DiskSpaceWarning():
return diskSpaceWarning(_that);case SyncEventFfi_InitialSyncComplete():
return initialSyncComplete(_that);case SyncEventFfi_WorkerStarted():
return workerStarted(_that);case SyncEventFfi_WorkerCompleted():
return workerCompleted(_that);case SyncEventFfi_WorkerFailed():
return workerFailed(_that);case SyncEventFfi_TaskItemUpdated():
return taskItemUpdated(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SyncEventFfi_StateChanged value)?  stateChanged,TResult? Function( SyncEventFfi_Progress value)?  progress,TResult? Function( SyncEventFfi_FileUploaded value)?  fileUploaded,TResult? Function( SyncEventFfi_FileDownloaded value)?  fileDownloaded,TResult? Function( SyncEventFfi_ConflictDetected value)?  conflictDetected,TResult? Function( SyncEventFfi_Error value)?  error,TResult? Function( SyncEventFfi_TokenExpired value)?  tokenExpired,TResult? Function( SyncEventFfi_DiskSpaceWarning value)?  diskSpaceWarning,TResult? Function( SyncEventFfi_InitialSyncComplete value)?  initialSyncComplete,TResult? Function( SyncEventFfi_WorkerStarted value)?  workerStarted,TResult? Function( SyncEventFfi_WorkerCompleted value)?  workerCompleted,TResult? Function( SyncEventFfi_WorkerFailed value)?  workerFailed,TResult? Function( SyncEventFfi_TaskItemUpdated value)?  taskItemUpdated,}){
final _that = this;
switch (_that) {
case SyncEventFfi_StateChanged() when stateChanged != null:
return stateChanged(_that);case SyncEventFfi_Progress() when progress != null:
return progress(_that);case SyncEventFfi_FileUploaded() when fileUploaded != null:
return fileUploaded(_that);case SyncEventFfi_FileDownloaded() when fileDownloaded != null:
return fileDownloaded(_that);case SyncEventFfi_ConflictDetected() when conflictDetected != null:
return conflictDetected(_that);case SyncEventFfi_Error() when error != null:
return error(_that);case SyncEventFfi_TokenExpired() when tokenExpired != null:
return tokenExpired(_that);case SyncEventFfi_DiskSpaceWarning() when diskSpaceWarning != null:
return diskSpaceWarning(_that);case SyncEventFfi_InitialSyncComplete() when initialSyncComplete != null:
return initialSyncComplete(_that);case SyncEventFfi_WorkerStarted() when workerStarted != null:
return workerStarted(_that);case SyncEventFfi_WorkerCompleted() when workerCompleted != null:
return workerCompleted(_that);case SyncEventFfi_WorkerFailed() when workerFailed != null:
return workerFailed(_that);case SyncEventFfi_TaskItemUpdated() when taskItemUpdated != null:
return taskItemUpdated(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String newState)?  stateChanged,TResult Function( BigInt synced,  BigInt total,  String currentFile)?  progress,TResult Function( String localPath,  String remoteUri)?  fileUploaded,TResult Function( String localPath,  String remoteUri)?  fileDownloaded,TResult Function( String localPath,  String conflictType)?  conflictDetected,TResult Function( String message,  bool recoverable)?  error,TResult Function()?  tokenExpired,TResult Function( BigInt availableMb)?  diskSpaceWarning,TResult Function( SyncSummaryFfi summary)?  initialSyncComplete,TResult Function( String taskId,  String trigger,  int uploadCount,  int downloadCount)?  workerStarted,TResult Function( String taskId,  int uploaded,  int downloaded,  int renamed,  int moved,  int failed,  BigInt durationMs)?  workerCompleted,TResult Function( String taskId,  String message)?  workerFailed,TResult Function( String taskId,  String relativePath,  String action,  String status)?  taskItemUpdated,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SyncEventFfi_StateChanged() when stateChanged != null:
return stateChanged(_that.newState);case SyncEventFfi_Progress() when progress != null:
return progress(_that.synced,_that.total,_that.currentFile);case SyncEventFfi_FileUploaded() when fileUploaded != null:
return fileUploaded(_that.localPath,_that.remoteUri);case SyncEventFfi_FileDownloaded() when fileDownloaded != null:
return fileDownloaded(_that.localPath,_that.remoteUri);case SyncEventFfi_ConflictDetected() when conflictDetected != null:
return conflictDetected(_that.localPath,_that.conflictType);case SyncEventFfi_Error() when error != null:
return error(_that.message,_that.recoverable);case SyncEventFfi_TokenExpired() when tokenExpired != null:
return tokenExpired();case SyncEventFfi_DiskSpaceWarning() when diskSpaceWarning != null:
return diskSpaceWarning(_that.availableMb);case SyncEventFfi_InitialSyncComplete() when initialSyncComplete != null:
return initialSyncComplete(_that.summary);case SyncEventFfi_WorkerStarted() when workerStarted != null:
return workerStarted(_that.taskId,_that.trigger,_that.uploadCount,_that.downloadCount);case SyncEventFfi_WorkerCompleted() when workerCompleted != null:
return workerCompleted(_that.taskId,_that.uploaded,_that.downloaded,_that.renamed,_that.moved,_that.failed,_that.durationMs);case SyncEventFfi_WorkerFailed() when workerFailed != null:
return workerFailed(_that.taskId,_that.message);case SyncEventFfi_TaskItemUpdated() when taskItemUpdated != null:
return taskItemUpdated(_that.taskId,_that.relativePath,_that.action,_that.status);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String newState)  stateChanged,required TResult Function( BigInt synced,  BigInt total,  String currentFile)  progress,required TResult Function( String localPath,  String remoteUri)  fileUploaded,required TResult Function( String localPath,  String remoteUri)  fileDownloaded,required TResult Function( String localPath,  String conflictType)  conflictDetected,required TResult Function( String message,  bool recoverable)  error,required TResult Function()  tokenExpired,required TResult Function( BigInt availableMb)  diskSpaceWarning,required TResult Function( SyncSummaryFfi summary)  initialSyncComplete,required TResult Function( String taskId,  String trigger,  int uploadCount,  int downloadCount)  workerStarted,required TResult Function( String taskId,  int uploaded,  int downloaded,  int renamed,  int moved,  int failed,  BigInt durationMs)  workerCompleted,required TResult Function( String taskId,  String message)  workerFailed,required TResult Function( String taskId,  String relativePath,  String action,  String status)  taskItemUpdated,}) {final _that = this;
switch (_that) {
case SyncEventFfi_StateChanged():
return stateChanged(_that.newState);case SyncEventFfi_Progress():
return progress(_that.synced,_that.total,_that.currentFile);case SyncEventFfi_FileUploaded():
return fileUploaded(_that.localPath,_that.remoteUri);case SyncEventFfi_FileDownloaded():
return fileDownloaded(_that.localPath,_that.remoteUri);case SyncEventFfi_ConflictDetected():
return conflictDetected(_that.localPath,_that.conflictType);case SyncEventFfi_Error():
return error(_that.message,_that.recoverable);case SyncEventFfi_TokenExpired():
return tokenExpired();case SyncEventFfi_DiskSpaceWarning():
return diskSpaceWarning(_that.availableMb);case SyncEventFfi_InitialSyncComplete():
return initialSyncComplete(_that.summary);case SyncEventFfi_WorkerStarted():
return workerStarted(_that.taskId,_that.trigger,_that.uploadCount,_that.downloadCount);case SyncEventFfi_WorkerCompleted():
return workerCompleted(_that.taskId,_that.uploaded,_that.downloaded,_that.renamed,_that.moved,_that.failed,_that.durationMs);case SyncEventFfi_WorkerFailed():
return workerFailed(_that.taskId,_that.message);case SyncEventFfi_TaskItemUpdated():
return taskItemUpdated(_that.taskId,_that.relativePath,_that.action,_that.status);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String newState)?  stateChanged,TResult? Function( BigInt synced,  BigInt total,  String currentFile)?  progress,TResult? Function( String localPath,  String remoteUri)?  fileUploaded,TResult? Function( String localPath,  String remoteUri)?  fileDownloaded,TResult? Function( String localPath,  String conflictType)?  conflictDetected,TResult? Function( String message,  bool recoverable)?  error,TResult? Function()?  tokenExpired,TResult? Function( BigInt availableMb)?  diskSpaceWarning,TResult? Function( SyncSummaryFfi summary)?  initialSyncComplete,TResult? Function( String taskId,  String trigger,  int uploadCount,  int downloadCount)?  workerStarted,TResult? Function( String taskId,  int uploaded,  int downloaded,  int renamed,  int moved,  int failed,  BigInt durationMs)?  workerCompleted,TResult? Function( String taskId,  String message)?  workerFailed,TResult? Function( String taskId,  String relativePath,  String action,  String status)?  taskItemUpdated,}) {final _that = this;
switch (_that) {
case SyncEventFfi_StateChanged() when stateChanged != null:
return stateChanged(_that.newState);case SyncEventFfi_Progress() when progress != null:
return progress(_that.synced,_that.total,_that.currentFile);case SyncEventFfi_FileUploaded() when fileUploaded != null:
return fileUploaded(_that.localPath,_that.remoteUri);case SyncEventFfi_FileDownloaded() when fileDownloaded != null:
return fileDownloaded(_that.localPath,_that.remoteUri);case SyncEventFfi_ConflictDetected() when conflictDetected != null:
return conflictDetected(_that.localPath,_that.conflictType);case SyncEventFfi_Error() when error != null:
return error(_that.message,_that.recoverable);case SyncEventFfi_TokenExpired() when tokenExpired != null:
return tokenExpired();case SyncEventFfi_DiskSpaceWarning() when diskSpaceWarning != null:
return diskSpaceWarning(_that.availableMb);case SyncEventFfi_InitialSyncComplete() when initialSyncComplete != null:
return initialSyncComplete(_that.summary);case SyncEventFfi_WorkerStarted() when workerStarted != null:
return workerStarted(_that.taskId,_that.trigger,_that.uploadCount,_that.downloadCount);case SyncEventFfi_WorkerCompleted() when workerCompleted != null:
return workerCompleted(_that.taskId,_that.uploaded,_that.downloaded,_that.renamed,_that.moved,_that.failed,_that.durationMs);case SyncEventFfi_WorkerFailed() when workerFailed != null:
return workerFailed(_that.taskId,_that.message);case SyncEventFfi_TaskItemUpdated() when taskItemUpdated != null:
return taskItemUpdated(_that.taskId,_that.relativePath,_that.action,_that.status);case _:
  return null;

}
}

}

/// @nodoc


class SyncEventFfi_StateChanged extends SyncEventFfi {
  const SyncEventFfi_StateChanged({required this.newState}): super._();
  

 final  String newState;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_StateChangedCopyWith<SyncEventFfi_StateChanged> get copyWith => _$SyncEventFfi_StateChangedCopyWithImpl<SyncEventFfi_StateChanged>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_StateChanged&&(identical(other.newState, newState) || other.newState == newState));
}


@override
int get hashCode => Object.hash(runtimeType,newState);

@override
String toString() {
  return 'SyncEventFfi.stateChanged(newState: $newState)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_StateChangedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_StateChangedCopyWith(SyncEventFfi_StateChanged value, $Res Function(SyncEventFfi_StateChanged) _then) = _$SyncEventFfi_StateChangedCopyWithImpl;
@useResult
$Res call({
 String newState
});




}
/// @nodoc
class _$SyncEventFfi_StateChangedCopyWithImpl<$Res>
    implements $SyncEventFfi_StateChangedCopyWith<$Res> {
  _$SyncEventFfi_StateChangedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_StateChanged _self;
  final $Res Function(SyncEventFfi_StateChanged) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? newState = null,}) {
  return _then(SyncEventFfi_StateChanged(
newState: null == newState ? _self.newState : newState // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncEventFfi_Progress extends SyncEventFfi {
  const SyncEventFfi_Progress({required this.synced, required this.total, required this.currentFile}): super._();
  

 final  BigInt synced;
 final  BigInt total;
 final  String currentFile;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_ProgressCopyWith<SyncEventFfi_Progress> get copyWith => _$SyncEventFfi_ProgressCopyWithImpl<SyncEventFfi_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_Progress&&(identical(other.synced, synced) || other.synced == synced)&&(identical(other.total, total) || other.total == total)&&(identical(other.currentFile, currentFile) || other.currentFile == currentFile));
}


@override
int get hashCode => Object.hash(runtimeType,synced,total,currentFile);

@override
String toString() {
  return 'SyncEventFfi.progress(synced: $synced, total: $total, currentFile: $currentFile)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_ProgressCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_ProgressCopyWith(SyncEventFfi_Progress value, $Res Function(SyncEventFfi_Progress) _then) = _$SyncEventFfi_ProgressCopyWithImpl;
@useResult
$Res call({
 BigInt synced, BigInt total, String currentFile
});




}
/// @nodoc
class _$SyncEventFfi_ProgressCopyWithImpl<$Res>
    implements $SyncEventFfi_ProgressCopyWith<$Res> {
  _$SyncEventFfi_ProgressCopyWithImpl(this._self, this._then);

  final SyncEventFfi_Progress _self;
  final $Res Function(SyncEventFfi_Progress) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? synced = null,Object? total = null,Object? currentFile = null,}) {
  return _then(SyncEventFfi_Progress(
synced: null == synced ? _self.synced : synced // ignore: cast_nullable_to_non_nullable
as BigInt,total: null == total ? _self.total : total // ignore: cast_nullable_to_non_nullable
as BigInt,currentFile: null == currentFile ? _self.currentFile : currentFile // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncEventFfi_FileUploaded extends SyncEventFfi {
  const SyncEventFfi_FileUploaded({required this.localPath, required this.remoteUri}): super._();
  

 final  String localPath;
 final  String remoteUri;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_FileUploadedCopyWith<SyncEventFfi_FileUploaded> get copyWith => _$SyncEventFfi_FileUploadedCopyWithImpl<SyncEventFfi_FileUploaded>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_FileUploaded&&(identical(other.localPath, localPath) || other.localPath == localPath)&&(identical(other.remoteUri, remoteUri) || other.remoteUri == remoteUri));
}


@override
int get hashCode => Object.hash(runtimeType,localPath,remoteUri);

@override
String toString() {
  return 'SyncEventFfi.fileUploaded(localPath: $localPath, remoteUri: $remoteUri)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_FileUploadedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_FileUploadedCopyWith(SyncEventFfi_FileUploaded value, $Res Function(SyncEventFfi_FileUploaded) _then) = _$SyncEventFfi_FileUploadedCopyWithImpl;
@useResult
$Res call({
 String localPath, String remoteUri
});




}
/// @nodoc
class _$SyncEventFfi_FileUploadedCopyWithImpl<$Res>
    implements $SyncEventFfi_FileUploadedCopyWith<$Res> {
  _$SyncEventFfi_FileUploadedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_FileUploaded _self;
  final $Res Function(SyncEventFfi_FileUploaded) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? localPath = null,Object? remoteUri = null,}) {
  return _then(SyncEventFfi_FileUploaded(
localPath: null == localPath ? _self.localPath : localPath // ignore: cast_nullable_to_non_nullable
as String,remoteUri: null == remoteUri ? _self.remoteUri : remoteUri // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncEventFfi_FileDownloaded extends SyncEventFfi {
  const SyncEventFfi_FileDownloaded({required this.localPath, required this.remoteUri}): super._();
  

 final  String localPath;
 final  String remoteUri;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_FileDownloadedCopyWith<SyncEventFfi_FileDownloaded> get copyWith => _$SyncEventFfi_FileDownloadedCopyWithImpl<SyncEventFfi_FileDownloaded>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_FileDownloaded&&(identical(other.localPath, localPath) || other.localPath == localPath)&&(identical(other.remoteUri, remoteUri) || other.remoteUri == remoteUri));
}


@override
int get hashCode => Object.hash(runtimeType,localPath,remoteUri);

@override
String toString() {
  return 'SyncEventFfi.fileDownloaded(localPath: $localPath, remoteUri: $remoteUri)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_FileDownloadedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_FileDownloadedCopyWith(SyncEventFfi_FileDownloaded value, $Res Function(SyncEventFfi_FileDownloaded) _then) = _$SyncEventFfi_FileDownloadedCopyWithImpl;
@useResult
$Res call({
 String localPath, String remoteUri
});




}
/// @nodoc
class _$SyncEventFfi_FileDownloadedCopyWithImpl<$Res>
    implements $SyncEventFfi_FileDownloadedCopyWith<$Res> {
  _$SyncEventFfi_FileDownloadedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_FileDownloaded _self;
  final $Res Function(SyncEventFfi_FileDownloaded) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? localPath = null,Object? remoteUri = null,}) {
  return _then(SyncEventFfi_FileDownloaded(
localPath: null == localPath ? _self.localPath : localPath // ignore: cast_nullable_to_non_nullable
as String,remoteUri: null == remoteUri ? _self.remoteUri : remoteUri // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncEventFfi_ConflictDetected extends SyncEventFfi {
  const SyncEventFfi_ConflictDetected({required this.localPath, required this.conflictType}): super._();
  

 final  String localPath;
 final  String conflictType;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_ConflictDetectedCopyWith<SyncEventFfi_ConflictDetected> get copyWith => _$SyncEventFfi_ConflictDetectedCopyWithImpl<SyncEventFfi_ConflictDetected>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_ConflictDetected&&(identical(other.localPath, localPath) || other.localPath == localPath)&&(identical(other.conflictType, conflictType) || other.conflictType == conflictType));
}


@override
int get hashCode => Object.hash(runtimeType,localPath,conflictType);

@override
String toString() {
  return 'SyncEventFfi.conflictDetected(localPath: $localPath, conflictType: $conflictType)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_ConflictDetectedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_ConflictDetectedCopyWith(SyncEventFfi_ConflictDetected value, $Res Function(SyncEventFfi_ConflictDetected) _then) = _$SyncEventFfi_ConflictDetectedCopyWithImpl;
@useResult
$Res call({
 String localPath, String conflictType
});




}
/// @nodoc
class _$SyncEventFfi_ConflictDetectedCopyWithImpl<$Res>
    implements $SyncEventFfi_ConflictDetectedCopyWith<$Res> {
  _$SyncEventFfi_ConflictDetectedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_ConflictDetected _self;
  final $Res Function(SyncEventFfi_ConflictDetected) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? localPath = null,Object? conflictType = null,}) {
  return _then(SyncEventFfi_ConflictDetected(
localPath: null == localPath ? _self.localPath : localPath // ignore: cast_nullable_to_non_nullable
as String,conflictType: null == conflictType ? _self.conflictType : conflictType // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncEventFfi_Error extends SyncEventFfi {
  const SyncEventFfi_Error({required this.message, required this.recoverable}): super._();
  

 final  String message;
 final  bool recoverable;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_ErrorCopyWith<SyncEventFfi_Error> get copyWith => _$SyncEventFfi_ErrorCopyWithImpl<SyncEventFfi_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_Error&&(identical(other.message, message) || other.message == message)&&(identical(other.recoverable, recoverable) || other.recoverable == recoverable));
}


@override
int get hashCode => Object.hash(runtimeType,message,recoverable);

@override
String toString() {
  return 'SyncEventFfi.error(message: $message, recoverable: $recoverable)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_ErrorCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_ErrorCopyWith(SyncEventFfi_Error value, $Res Function(SyncEventFfi_Error) _then) = _$SyncEventFfi_ErrorCopyWithImpl;
@useResult
$Res call({
 String message, bool recoverable
});




}
/// @nodoc
class _$SyncEventFfi_ErrorCopyWithImpl<$Res>
    implements $SyncEventFfi_ErrorCopyWith<$Res> {
  _$SyncEventFfi_ErrorCopyWithImpl(this._self, this._then);

  final SyncEventFfi_Error _self;
  final $Res Function(SyncEventFfi_Error) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? recoverable = null,}) {
  return _then(SyncEventFfi_Error(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,recoverable: null == recoverable ? _self.recoverable : recoverable // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class SyncEventFfi_TokenExpired extends SyncEventFfi {
  const SyncEventFfi_TokenExpired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_TokenExpired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncEventFfi.tokenExpired()';
}


}




/// @nodoc


class SyncEventFfi_DiskSpaceWarning extends SyncEventFfi {
  const SyncEventFfi_DiskSpaceWarning({required this.availableMb}): super._();
  

 final  BigInt availableMb;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_DiskSpaceWarningCopyWith<SyncEventFfi_DiskSpaceWarning> get copyWith => _$SyncEventFfi_DiskSpaceWarningCopyWithImpl<SyncEventFfi_DiskSpaceWarning>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_DiskSpaceWarning&&(identical(other.availableMb, availableMb) || other.availableMb == availableMb));
}


@override
int get hashCode => Object.hash(runtimeType,availableMb);

@override
String toString() {
  return 'SyncEventFfi.diskSpaceWarning(availableMb: $availableMb)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_DiskSpaceWarningCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_DiskSpaceWarningCopyWith(SyncEventFfi_DiskSpaceWarning value, $Res Function(SyncEventFfi_DiskSpaceWarning) _then) = _$SyncEventFfi_DiskSpaceWarningCopyWithImpl;
@useResult
$Res call({
 BigInt availableMb
});




}
/// @nodoc
class _$SyncEventFfi_DiskSpaceWarningCopyWithImpl<$Res>
    implements $SyncEventFfi_DiskSpaceWarningCopyWith<$Res> {
  _$SyncEventFfi_DiskSpaceWarningCopyWithImpl(this._self, this._then);

  final SyncEventFfi_DiskSpaceWarning _self;
  final $Res Function(SyncEventFfi_DiskSpaceWarning) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? availableMb = null,}) {
  return _then(SyncEventFfi_DiskSpaceWarning(
availableMb: null == availableMb ? _self.availableMb : availableMb // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class SyncEventFfi_InitialSyncComplete extends SyncEventFfi {
  const SyncEventFfi_InitialSyncComplete({required this.summary}): super._();
  

 final  SyncSummaryFfi summary;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_InitialSyncCompleteCopyWith<SyncEventFfi_InitialSyncComplete> get copyWith => _$SyncEventFfi_InitialSyncCompleteCopyWithImpl<SyncEventFfi_InitialSyncComplete>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_InitialSyncComplete&&(identical(other.summary, summary) || other.summary == summary));
}


@override
int get hashCode => Object.hash(runtimeType,summary);

@override
String toString() {
  return 'SyncEventFfi.initialSyncComplete(summary: $summary)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_InitialSyncCompleteCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_InitialSyncCompleteCopyWith(SyncEventFfi_InitialSyncComplete value, $Res Function(SyncEventFfi_InitialSyncComplete) _then) = _$SyncEventFfi_InitialSyncCompleteCopyWithImpl;
@useResult
$Res call({
 SyncSummaryFfi summary
});




}
/// @nodoc
class _$SyncEventFfi_InitialSyncCompleteCopyWithImpl<$Res>
    implements $SyncEventFfi_InitialSyncCompleteCopyWith<$Res> {
  _$SyncEventFfi_InitialSyncCompleteCopyWithImpl(this._self, this._then);

  final SyncEventFfi_InitialSyncComplete _self;
  final $Res Function(SyncEventFfi_InitialSyncComplete) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? summary = null,}) {
  return _then(SyncEventFfi_InitialSyncComplete(
summary: null == summary ? _self.summary : summary // ignore: cast_nullable_to_non_nullable
as SyncSummaryFfi,
  ));
}


}

/// @nodoc


class SyncEventFfi_WorkerStarted extends SyncEventFfi {
  const SyncEventFfi_WorkerStarted({required this.taskId, required this.trigger, required this.uploadCount, required this.downloadCount}): super._();
  

 final  String taskId;
 final  String trigger;
 final  int uploadCount;
 final  int downloadCount;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_WorkerStartedCopyWith<SyncEventFfi_WorkerStarted> get copyWith => _$SyncEventFfi_WorkerStartedCopyWithImpl<SyncEventFfi_WorkerStarted>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_WorkerStarted&&(identical(other.taskId, taskId) || other.taskId == taskId)&&(identical(other.trigger, trigger) || other.trigger == trigger)&&(identical(other.uploadCount, uploadCount) || other.uploadCount == uploadCount)&&(identical(other.downloadCount, downloadCount) || other.downloadCount == downloadCount));
}


@override
int get hashCode => Object.hash(runtimeType,taskId,trigger,uploadCount,downloadCount);

@override
String toString() {
  return 'SyncEventFfi.workerStarted(taskId: $taskId, trigger: $trigger, uploadCount: $uploadCount, downloadCount: $downloadCount)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_WorkerStartedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_WorkerStartedCopyWith(SyncEventFfi_WorkerStarted value, $Res Function(SyncEventFfi_WorkerStarted) _then) = _$SyncEventFfi_WorkerStartedCopyWithImpl;
@useResult
$Res call({
 String taskId, String trigger, int uploadCount, int downloadCount
});




}
/// @nodoc
class _$SyncEventFfi_WorkerStartedCopyWithImpl<$Res>
    implements $SyncEventFfi_WorkerStartedCopyWith<$Res> {
  _$SyncEventFfi_WorkerStartedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_WorkerStarted _self;
  final $Res Function(SyncEventFfi_WorkerStarted) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? taskId = null,Object? trigger = null,Object? uploadCount = null,Object? downloadCount = null,}) {
  return _then(SyncEventFfi_WorkerStarted(
taskId: null == taskId ? _self.taskId : taskId // ignore: cast_nullable_to_non_nullable
as String,trigger: null == trigger ? _self.trigger : trigger // ignore: cast_nullable_to_non_nullable
as String,uploadCount: null == uploadCount ? _self.uploadCount : uploadCount // ignore: cast_nullable_to_non_nullable
as int,downloadCount: null == downloadCount ? _self.downloadCount : downloadCount // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SyncEventFfi_WorkerCompleted extends SyncEventFfi {
  const SyncEventFfi_WorkerCompleted({required this.taskId, required this.uploaded, required this.downloaded, required this.renamed, required this.moved, required this.failed, required this.durationMs}): super._();
  

 final  String taskId;
 final  int uploaded;
 final  int downloaded;
 final  int renamed;
 final  int moved;
 final  int failed;
 final  BigInt durationMs;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_WorkerCompletedCopyWith<SyncEventFfi_WorkerCompleted> get copyWith => _$SyncEventFfi_WorkerCompletedCopyWithImpl<SyncEventFfi_WorkerCompleted>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_WorkerCompleted&&(identical(other.taskId, taskId) || other.taskId == taskId)&&(identical(other.uploaded, uploaded) || other.uploaded == uploaded)&&(identical(other.downloaded, downloaded) || other.downloaded == downloaded)&&(identical(other.renamed, renamed) || other.renamed == renamed)&&(identical(other.moved, moved) || other.moved == moved)&&(identical(other.failed, failed) || other.failed == failed)&&(identical(other.durationMs, durationMs) || other.durationMs == durationMs));
}


@override
int get hashCode => Object.hash(runtimeType,taskId,uploaded,downloaded,renamed,moved,failed,durationMs);

@override
String toString() {
  return 'SyncEventFfi.workerCompleted(taskId: $taskId, uploaded: $uploaded, downloaded: $downloaded, renamed: $renamed, moved: $moved, failed: $failed, durationMs: $durationMs)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_WorkerCompletedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_WorkerCompletedCopyWith(SyncEventFfi_WorkerCompleted value, $Res Function(SyncEventFfi_WorkerCompleted) _then) = _$SyncEventFfi_WorkerCompletedCopyWithImpl;
@useResult
$Res call({
 String taskId, int uploaded, int downloaded, int renamed, int moved, int failed, BigInt durationMs
});




}
/// @nodoc
class _$SyncEventFfi_WorkerCompletedCopyWithImpl<$Res>
    implements $SyncEventFfi_WorkerCompletedCopyWith<$Res> {
  _$SyncEventFfi_WorkerCompletedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_WorkerCompleted _self;
  final $Res Function(SyncEventFfi_WorkerCompleted) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? taskId = null,Object? uploaded = null,Object? downloaded = null,Object? renamed = null,Object? moved = null,Object? failed = null,Object? durationMs = null,}) {
  return _then(SyncEventFfi_WorkerCompleted(
taskId: null == taskId ? _self.taskId : taskId // ignore: cast_nullable_to_non_nullable
as String,uploaded: null == uploaded ? _self.uploaded : uploaded // ignore: cast_nullable_to_non_nullable
as int,downloaded: null == downloaded ? _self.downloaded : downloaded // ignore: cast_nullable_to_non_nullable
as int,renamed: null == renamed ? _self.renamed : renamed // ignore: cast_nullable_to_non_nullable
as int,moved: null == moved ? _self.moved : moved // ignore: cast_nullable_to_non_nullable
as int,failed: null == failed ? _self.failed : failed // ignore: cast_nullable_to_non_nullable
as int,durationMs: null == durationMs ? _self.durationMs : durationMs // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class SyncEventFfi_WorkerFailed extends SyncEventFfi {
  const SyncEventFfi_WorkerFailed({required this.taskId, required this.message}): super._();
  

 final  String taskId;
 final  String message;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_WorkerFailedCopyWith<SyncEventFfi_WorkerFailed> get copyWith => _$SyncEventFfi_WorkerFailedCopyWithImpl<SyncEventFfi_WorkerFailed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_WorkerFailed&&(identical(other.taskId, taskId) || other.taskId == taskId)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,taskId,message);

@override
String toString() {
  return 'SyncEventFfi.workerFailed(taskId: $taskId, message: $message)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_WorkerFailedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_WorkerFailedCopyWith(SyncEventFfi_WorkerFailed value, $Res Function(SyncEventFfi_WorkerFailed) _then) = _$SyncEventFfi_WorkerFailedCopyWithImpl;
@useResult
$Res call({
 String taskId, String message
});




}
/// @nodoc
class _$SyncEventFfi_WorkerFailedCopyWithImpl<$Res>
    implements $SyncEventFfi_WorkerFailedCopyWith<$Res> {
  _$SyncEventFfi_WorkerFailedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_WorkerFailed _self;
  final $Res Function(SyncEventFfi_WorkerFailed) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? taskId = null,Object? message = null,}) {
  return _then(SyncEventFfi_WorkerFailed(
taskId: null == taskId ? _self.taskId : taskId // ignore: cast_nullable_to_non_nullable
as String,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncEventFfi_TaskItemUpdated extends SyncEventFfi {
  const SyncEventFfi_TaskItemUpdated({required this.taskId, required this.relativePath, required this.action, required this.status}): super._();
  

 final  String taskId;
 final  String relativePath;
 final  String action;
 final  String status;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEventFfi_TaskItemUpdatedCopyWith<SyncEventFfi_TaskItemUpdated> get copyWith => _$SyncEventFfi_TaskItemUpdatedCopyWithImpl<SyncEventFfi_TaskItemUpdated>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEventFfi_TaskItemUpdated&&(identical(other.taskId, taskId) || other.taskId == taskId)&&(identical(other.relativePath, relativePath) || other.relativePath == relativePath)&&(identical(other.action, action) || other.action == action)&&(identical(other.status, status) || other.status == status));
}


@override
int get hashCode => Object.hash(runtimeType,taskId,relativePath,action,status);

@override
String toString() {
  return 'SyncEventFfi.taskItemUpdated(taskId: $taskId, relativePath: $relativePath, action: $action, status: $status)';
}


}

/// @nodoc
abstract mixin class $SyncEventFfi_TaskItemUpdatedCopyWith<$Res> implements $SyncEventFfiCopyWith<$Res> {
  factory $SyncEventFfi_TaskItemUpdatedCopyWith(SyncEventFfi_TaskItemUpdated value, $Res Function(SyncEventFfi_TaskItemUpdated) _then) = _$SyncEventFfi_TaskItemUpdatedCopyWithImpl;
@useResult
$Res call({
 String taskId, String relativePath, String action, String status
});




}
/// @nodoc
class _$SyncEventFfi_TaskItemUpdatedCopyWithImpl<$Res>
    implements $SyncEventFfi_TaskItemUpdatedCopyWith<$Res> {
  _$SyncEventFfi_TaskItemUpdatedCopyWithImpl(this._self, this._then);

  final SyncEventFfi_TaskItemUpdated _self;
  final $Res Function(SyncEventFfi_TaskItemUpdated) _then;

/// Create a copy of SyncEventFfi
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? taskId = null,Object? relativePath = null,Object? action = null,Object? status = null,}) {
  return _then(SyncEventFfi_TaskItemUpdated(
taskId: null == taskId ? _self.taskId : taskId // ignore: cast_nullable_to_non_nullable
as String,relativePath: null == relativePath ? _self.relativePath : relativePath // ignore: cast_nullable_to_non_nullable
as String,action: null == action ? _self.action : action // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
