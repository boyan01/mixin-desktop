// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'error.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CoreError {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'CoreError()';
}


}

/// @nodoc
class $CoreErrorCopyWith<$Res>  {
$CoreErrorCopyWith(CoreError _, $Res Function(CoreError) __);
}


/// Adds pattern-matching-related methods to [CoreError].
extension CoreErrorPatterns on CoreError {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CoreError_Unauthorized value)?  unauthorized,TResult Function( CoreError_NotFound value)?  notFound,TResult Function( CoreError_Cancelled value)?  cancelled,TResult Function( CoreError_InvalidArgument value)?  invalidArgument,TResult Function( CoreError_Other value)?  other,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CoreError_Unauthorized() when unauthorized != null:
return unauthorized(_that);case CoreError_NotFound() when notFound != null:
return notFound(_that);case CoreError_Cancelled() when cancelled != null:
return cancelled(_that);case CoreError_InvalidArgument() when invalidArgument != null:
return invalidArgument(_that);case CoreError_Other() when other != null:
return other(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CoreError_Unauthorized value)  unauthorized,required TResult Function( CoreError_NotFound value)  notFound,required TResult Function( CoreError_Cancelled value)  cancelled,required TResult Function( CoreError_InvalidArgument value)  invalidArgument,required TResult Function( CoreError_Other value)  other,}){
final _that = this;
switch (_that) {
case CoreError_Unauthorized():
return unauthorized(_that);case CoreError_NotFound():
return notFound(_that);case CoreError_Cancelled():
return cancelled(_that);case CoreError_InvalidArgument():
return invalidArgument(_that);case CoreError_Other():
return other(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CoreError_Unauthorized value)?  unauthorized,TResult? Function( CoreError_NotFound value)?  notFound,TResult? Function( CoreError_Cancelled value)?  cancelled,TResult? Function( CoreError_InvalidArgument value)?  invalidArgument,TResult? Function( CoreError_Other value)?  other,}){
final _that = this;
switch (_that) {
case CoreError_Unauthorized() when unauthorized != null:
return unauthorized(_that);case CoreError_NotFound() when notFound != null:
return notFound(_that);case CoreError_Cancelled() when cancelled != null:
return cancelled(_that);case CoreError_InvalidArgument() when invalidArgument != null:
return invalidArgument(_that);case CoreError_Other() when other != null:
return other(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  unauthorized,TResult Function()?  notFound,TResult Function()?  cancelled,TResult Function( String message)?  invalidArgument,TResult Function( String message)?  other,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CoreError_Unauthorized() when unauthorized != null:
return unauthorized();case CoreError_NotFound() when notFound != null:
return notFound();case CoreError_Cancelled() when cancelled != null:
return cancelled();case CoreError_InvalidArgument() when invalidArgument != null:
return invalidArgument(_that.message);case CoreError_Other() when other != null:
return other(_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  unauthorized,required TResult Function()  notFound,required TResult Function()  cancelled,required TResult Function( String message)  invalidArgument,required TResult Function( String message)  other,}) {final _that = this;
switch (_that) {
case CoreError_Unauthorized():
return unauthorized();case CoreError_NotFound():
return notFound();case CoreError_Cancelled():
return cancelled();case CoreError_InvalidArgument():
return invalidArgument(_that.message);case CoreError_Other():
return other(_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  unauthorized,TResult? Function()?  notFound,TResult? Function()?  cancelled,TResult? Function( String message)?  invalidArgument,TResult? Function( String message)?  other,}) {final _that = this;
switch (_that) {
case CoreError_Unauthorized() when unauthorized != null:
return unauthorized();case CoreError_NotFound() when notFound != null:
return notFound();case CoreError_Cancelled() when cancelled != null:
return cancelled();case CoreError_InvalidArgument() when invalidArgument != null:
return invalidArgument(_that.message);case CoreError_Other() when other != null:
return other(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class CoreError_Unauthorized extends CoreError {
  const CoreError_Unauthorized(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreError_Unauthorized);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'CoreError.unauthorized()';
}


}




/// @nodoc


class CoreError_NotFound extends CoreError {
  const CoreError_NotFound(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreError_NotFound);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'CoreError.notFound()';
}


}




/// @nodoc


class CoreError_Cancelled extends CoreError {
  const CoreError_Cancelled(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreError_Cancelled);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'CoreError.cancelled()';
}


}




/// @nodoc


class CoreError_InvalidArgument extends CoreError {
  const CoreError_InvalidArgument({required this.message}): super._();


 final  String message;

/// Create a copy of CoreError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreError_InvalidArgumentCopyWith<CoreError_InvalidArgument> get copyWith => _$CoreError_InvalidArgumentCopyWithImpl<CoreError_InvalidArgument>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreError_InvalidArgument&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'CoreError.invalidArgument(message: $message)';
}


}

/// @nodoc
abstract mixin class $CoreError_InvalidArgumentCopyWith<$Res> implements $CoreErrorCopyWith<$Res> {
  factory $CoreError_InvalidArgumentCopyWith(CoreError_InvalidArgument value, $Res Function(CoreError_InvalidArgument) _then) = _$CoreError_InvalidArgumentCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$CoreError_InvalidArgumentCopyWithImpl<$Res>
    implements $CoreError_InvalidArgumentCopyWith<$Res> {
  _$CoreError_InvalidArgumentCopyWithImpl(this._self, this._then);

  final CoreError_InvalidArgument _self;
  final $Res Function(CoreError_InvalidArgument) _then;

/// Create a copy of CoreError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(CoreError_InvalidArgument(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CoreError_Other extends CoreError {
  const CoreError_Other({required this.message}): super._();


 final  String message;

/// Create a copy of CoreError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreError_OtherCopyWith<CoreError_Other> get copyWith => _$CoreError_OtherCopyWithImpl<CoreError_Other>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreError_Other&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'CoreError.other(message: $message)';
}


}

/// @nodoc
abstract mixin class $CoreError_OtherCopyWith<$Res> implements $CoreErrorCopyWith<$Res> {
  factory $CoreError_OtherCopyWith(CoreError_Other value, $Res Function(CoreError_Other) _then) = _$CoreError_OtherCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$CoreError_OtherCopyWithImpl<$Res>
    implements $CoreError_OtherCopyWith<$Res> {
  _$CoreError_OtherCopyWithImpl(this._self, this._then);

  final CoreError_Other _self;
  final $Res Function(CoreError_Other) _then;

/// Create a copy of CoreError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(CoreError_Other(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
