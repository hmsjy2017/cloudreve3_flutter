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

// dart format on
