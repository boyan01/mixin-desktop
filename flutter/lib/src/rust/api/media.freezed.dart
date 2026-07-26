// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'media.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$MediaPlaybackEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaPlaybackEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'MediaPlaybackEvent()';
}


}

/// @nodoc
class $MediaPlaybackEventCopyWith<$Res>  {
$MediaPlaybackEventCopyWith(MediaPlaybackEvent _, $Res Function(MediaPlaybackEvent) __);
}


/// Adds pattern-matching-related methods to [MediaPlaybackEvent].
extension MediaPlaybackEventPatterns on MediaPlaybackEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MediaPlaybackEvent_Changed value)?  changed,TResult Function( MediaPlaybackEvent_Finished value)?  finished,TResult Function( MediaPlaybackEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MediaPlaybackEvent_Changed() when changed != null:
return changed(_that);case MediaPlaybackEvent_Finished() when finished != null:
return finished(_that);case MediaPlaybackEvent_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MediaPlaybackEvent_Changed value)  changed,required TResult Function( MediaPlaybackEvent_Finished value)  finished,required TResult Function( MediaPlaybackEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case MediaPlaybackEvent_Changed():
return changed(_that);case MediaPlaybackEvent_Finished():
return finished(_that);case MediaPlaybackEvent_Failed():
return failed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MediaPlaybackEvent_Changed value)?  changed,TResult? Function( MediaPlaybackEvent_Finished value)?  finished,TResult? Function( MediaPlaybackEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case MediaPlaybackEvent_Changed() when changed != null:
return changed(_that);case MediaPlaybackEvent_Finished() when finished != null:
return finished(_that);case MediaPlaybackEvent_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( MediaPlaybackSnapshot snapshot)?  changed,TResult Function( String id)?  finished,TResult Function( String? id,  String message)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MediaPlaybackEvent_Changed() when changed != null:
return changed(_that.snapshot);case MediaPlaybackEvent_Finished() when finished != null:
return finished(_that.id);case MediaPlaybackEvent_Failed() when failed != null:
return failed(_that.id,_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( MediaPlaybackSnapshot snapshot)  changed,required TResult Function( String id)  finished,required TResult Function( String? id,  String message)  failed,}) {final _that = this;
switch (_that) {
case MediaPlaybackEvent_Changed():
return changed(_that.snapshot);case MediaPlaybackEvent_Finished():
return finished(_that.id);case MediaPlaybackEvent_Failed():
return failed(_that.id,_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( MediaPlaybackSnapshot snapshot)?  changed,TResult? Function( String id)?  finished,TResult? Function( String? id,  String message)?  failed,}) {final _that = this;
switch (_that) {
case MediaPlaybackEvent_Changed() when changed != null:
return changed(_that.snapshot);case MediaPlaybackEvent_Finished() when finished != null:
return finished(_that.id);case MediaPlaybackEvent_Failed() when failed != null:
return failed(_that.id,_that.message);case _:
  return null;

}
}

}

/// @nodoc


class MediaPlaybackEvent_Changed extends MediaPlaybackEvent {
  const MediaPlaybackEvent_Changed({required this.snapshot}): super._();


 final  MediaPlaybackSnapshot snapshot;

/// Create a copy of MediaPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MediaPlaybackEvent_ChangedCopyWith<MediaPlaybackEvent_Changed> get copyWith => _$MediaPlaybackEvent_ChangedCopyWithImpl<MediaPlaybackEvent_Changed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaPlaybackEvent_Changed&&(identical(other.snapshot, snapshot) || other.snapshot == snapshot));
}


@override
int get hashCode => Object.hash(runtimeType,snapshot);

@override
String toString() {
  return 'MediaPlaybackEvent.changed(snapshot: $snapshot)';
}


}

/// @nodoc
abstract mixin class $MediaPlaybackEvent_ChangedCopyWith<$Res> implements $MediaPlaybackEventCopyWith<$Res> {
  factory $MediaPlaybackEvent_ChangedCopyWith(MediaPlaybackEvent_Changed value, $Res Function(MediaPlaybackEvent_Changed) _then) = _$MediaPlaybackEvent_ChangedCopyWithImpl;
@useResult
$Res call({
 MediaPlaybackSnapshot snapshot
});




}
/// @nodoc
class _$MediaPlaybackEvent_ChangedCopyWithImpl<$Res>
    implements $MediaPlaybackEvent_ChangedCopyWith<$Res> {
  _$MediaPlaybackEvent_ChangedCopyWithImpl(this._self, this._then);

  final MediaPlaybackEvent_Changed _self;
  final $Res Function(MediaPlaybackEvent_Changed) _then;

/// Create a copy of MediaPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? snapshot = null,}) {
  return _then(MediaPlaybackEvent_Changed(
snapshot: null == snapshot ? _self.snapshot : snapshot // ignore: cast_nullable_to_non_nullable
as MediaPlaybackSnapshot,
  ));
}


}

/// @nodoc


class MediaPlaybackEvent_Finished extends MediaPlaybackEvent {
  const MediaPlaybackEvent_Finished({required this.id}): super._();


 final  String id;

/// Create a copy of MediaPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MediaPlaybackEvent_FinishedCopyWith<MediaPlaybackEvent_Finished> get copyWith => _$MediaPlaybackEvent_FinishedCopyWithImpl<MediaPlaybackEvent_Finished>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaPlaybackEvent_Finished&&(identical(other.id, id) || other.id == id));
}


@override
int get hashCode => Object.hash(runtimeType,id);

@override
String toString() {
  return 'MediaPlaybackEvent.finished(id: $id)';
}


}

/// @nodoc
abstract mixin class $MediaPlaybackEvent_FinishedCopyWith<$Res> implements $MediaPlaybackEventCopyWith<$Res> {
  factory $MediaPlaybackEvent_FinishedCopyWith(MediaPlaybackEvent_Finished value, $Res Function(MediaPlaybackEvent_Finished) _then) = _$MediaPlaybackEvent_FinishedCopyWithImpl;
@useResult
$Res call({
 String id
});




}
/// @nodoc
class _$MediaPlaybackEvent_FinishedCopyWithImpl<$Res>
    implements $MediaPlaybackEvent_FinishedCopyWith<$Res> {
  _$MediaPlaybackEvent_FinishedCopyWithImpl(this._self, this._then);

  final MediaPlaybackEvent_Finished _self;
  final $Res Function(MediaPlaybackEvent_Finished) _then;

/// Create a copy of MediaPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? id = null,}) {
  return _then(MediaPlaybackEvent_Finished(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class MediaPlaybackEvent_Failed extends MediaPlaybackEvent {
  const MediaPlaybackEvent_Failed({this.id, required this.message}): super._();


 final  String? id;
 final  String message;

/// Create a copy of MediaPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MediaPlaybackEvent_FailedCopyWith<MediaPlaybackEvent_Failed> get copyWith => _$MediaPlaybackEvent_FailedCopyWithImpl<MediaPlaybackEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaPlaybackEvent_Failed&&(identical(other.id, id) || other.id == id)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,id,message);

@override
String toString() {
  return 'MediaPlaybackEvent.failed(id: $id, message: $message)';
}


}

/// @nodoc
abstract mixin class $MediaPlaybackEvent_FailedCopyWith<$Res> implements $MediaPlaybackEventCopyWith<$Res> {
  factory $MediaPlaybackEvent_FailedCopyWith(MediaPlaybackEvent_Failed value, $Res Function(MediaPlaybackEvent_Failed) _then) = _$MediaPlaybackEvent_FailedCopyWithImpl;
@useResult
$Res call({
 String? id, String message
});




}
/// @nodoc
class _$MediaPlaybackEvent_FailedCopyWithImpl<$Res>
    implements $MediaPlaybackEvent_FailedCopyWith<$Res> {
  _$MediaPlaybackEvent_FailedCopyWithImpl(this._self, this._then);

  final MediaPlaybackEvent_Failed _self;
  final $Res Function(MediaPlaybackEvent_Failed) _then;

/// Create a copy of MediaPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? id = freezed,Object? message = null,}) {
  return _then(MediaPlaybackEvent_Failed(
id: freezed == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String?,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$MediaRecorderEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaRecorderEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'MediaRecorderEvent()';
}


}

/// @nodoc
class $MediaRecorderEventCopyWith<$Res>  {
$MediaRecorderEventCopyWith(MediaRecorderEvent _, $Res Function(MediaRecorderEvent) __);
}


/// Adds pattern-matching-related methods to [MediaRecorderEvent].
extension MediaRecorderEventPatterns on MediaRecorderEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MediaRecorderEvent_Changed value)?  changed,TResult Function( MediaRecorderEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MediaRecorderEvent_Changed() when changed != null:
return changed(_that);case MediaRecorderEvent_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MediaRecorderEvent_Changed value)  changed,required TResult Function( MediaRecorderEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case MediaRecorderEvent_Changed():
return changed(_that);case MediaRecorderEvent_Failed():
return failed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MediaRecorderEvent_Changed value)?  changed,TResult? Function( MediaRecorderEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case MediaRecorderEvent_Changed() when changed != null:
return changed(_that);case MediaRecorderEvent_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( MediaRecorderSnapshot snapshot)?  changed,TResult Function( String message)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MediaRecorderEvent_Changed() when changed != null:
return changed(_that.snapshot);case MediaRecorderEvent_Failed() when failed != null:
return failed(_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( MediaRecorderSnapshot snapshot)  changed,required TResult Function( String message)  failed,}) {final _that = this;
switch (_that) {
case MediaRecorderEvent_Changed():
return changed(_that.snapshot);case MediaRecorderEvent_Failed():
return failed(_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( MediaRecorderSnapshot snapshot)?  changed,TResult? Function( String message)?  failed,}) {final _that = this;
switch (_that) {
case MediaRecorderEvent_Changed() when changed != null:
return changed(_that.snapshot);case MediaRecorderEvent_Failed() when failed != null:
return failed(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class MediaRecorderEvent_Changed extends MediaRecorderEvent {
  const MediaRecorderEvent_Changed({required this.snapshot}): super._();


 final  MediaRecorderSnapshot snapshot;

/// Create a copy of MediaRecorderEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MediaRecorderEvent_ChangedCopyWith<MediaRecorderEvent_Changed> get copyWith => _$MediaRecorderEvent_ChangedCopyWithImpl<MediaRecorderEvent_Changed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaRecorderEvent_Changed&&(identical(other.snapshot, snapshot) || other.snapshot == snapshot));
}


@override
int get hashCode => Object.hash(runtimeType,snapshot);

@override
String toString() {
  return 'MediaRecorderEvent.changed(snapshot: $snapshot)';
}


}

/// @nodoc
abstract mixin class $MediaRecorderEvent_ChangedCopyWith<$Res> implements $MediaRecorderEventCopyWith<$Res> {
  factory $MediaRecorderEvent_ChangedCopyWith(MediaRecorderEvent_Changed value, $Res Function(MediaRecorderEvent_Changed) _then) = _$MediaRecorderEvent_ChangedCopyWithImpl;
@useResult
$Res call({
 MediaRecorderSnapshot snapshot
});




}
/// @nodoc
class _$MediaRecorderEvent_ChangedCopyWithImpl<$Res>
    implements $MediaRecorderEvent_ChangedCopyWith<$Res> {
  _$MediaRecorderEvent_ChangedCopyWithImpl(this._self, this._then);

  final MediaRecorderEvent_Changed _self;
  final $Res Function(MediaRecorderEvent_Changed) _then;

/// Create a copy of MediaRecorderEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? snapshot = null,}) {
  return _then(MediaRecorderEvent_Changed(
snapshot: null == snapshot ? _self.snapshot : snapshot // ignore: cast_nullable_to_non_nullable
as MediaRecorderSnapshot,
  ));
}


}

/// @nodoc


class MediaRecorderEvent_Failed extends MediaRecorderEvent {
  const MediaRecorderEvent_Failed({required this.message}): super._();


 final  String message;

/// Create a copy of MediaRecorderEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MediaRecorderEvent_FailedCopyWith<MediaRecorderEvent_Failed> get copyWith => _$MediaRecorderEvent_FailedCopyWithImpl<MediaRecorderEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MediaRecorderEvent_Failed&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'MediaRecorderEvent.failed(message: $message)';
}


}

/// @nodoc
abstract mixin class $MediaRecorderEvent_FailedCopyWith<$Res> implements $MediaRecorderEventCopyWith<$Res> {
  factory $MediaRecorderEvent_FailedCopyWith(MediaRecorderEvent_Failed value, $Res Function(MediaRecorderEvent_Failed) _then) = _$MediaRecorderEvent_FailedCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$MediaRecorderEvent_FailedCopyWithImpl<$Res>
    implements $MediaRecorderEvent_FailedCopyWith<$Res> {
  _$MediaRecorderEvent_FailedCopyWithImpl(this._self, this._then);

  final MediaRecorderEvent_Failed _self;
  final $Res Function(MediaRecorderEvent_Failed) _then;

/// Create a copy of MediaRecorderEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(MediaRecorderEvent_Failed(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
