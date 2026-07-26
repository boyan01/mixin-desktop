// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'device_transfer.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$DeviceTransferEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent()';
}


}

/// @nodoc
class $DeviceTransferEventCopyWith<$Res>  {
$DeviceTransferEventCopyWith(DeviceTransferEvent _, $Res Function(DeviceTransferEvent) __);
}


/// Adds pattern-matching-related methods to [DeviceTransferEvent].
extension DeviceTransferEventPatterns on DeviceTransferEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( DeviceTransferEvent_RestoreConnected value)?  restoreConnected,TResult Function( DeviceTransferEvent_RestoreStart value)?  restoreStart,TResult Function( DeviceTransferEvent_RestoreSucceed value)?  restoreSucceed,TResult Function( DeviceTransferEvent_RestoreFailed value)?  restoreFailed,TResult Function( DeviceTransferEvent_BackupServerCreated value)?  backupServerCreated,TResult Function( DeviceTransferEvent_BackupStart value)?  backupStart,TResult Function( DeviceTransferEvent_BackupSucceed value)?  backupSucceed,TResult Function( DeviceTransferEvent_BackupFailed value)?  backupFailed,TResult Function( DeviceTransferEvent_RestoreProgress value)?  restoreProgress,TResult Function( DeviceTransferEvent_BackupProgress value)?  backupProgress,TResult Function( DeviceTransferEvent_RestoreNetworkSpeed value)?  restoreNetworkSpeed,TResult Function( DeviceTransferEvent_BackupNetworkSpeed value)?  backupNetworkSpeed,TResult Function( DeviceTransferEvent_BackupRequestReceived value)?  backupRequestReceived,TResult Function( DeviceTransferEvent_RestoreRequestReceived value)?  restoreRequestReceived,TResult Function( DeviceTransferEvent_ConnectionFailed value)?  connectionFailed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case DeviceTransferEvent_RestoreConnected() when restoreConnected != null:
return restoreConnected(_that);case DeviceTransferEvent_RestoreStart() when restoreStart != null:
return restoreStart(_that);case DeviceTransferEvent_RestoreSucceed() when restoreSucceed != null:
return restoreSucceed(_that);case DeviceTransferEvent_RestoreFailed() when restoreFailed != null:
return restoreFailed(_that);case DeviceTransferEvent_BackupServerCreated() when backupServerCreated != null:
return backupServerCreated(_that);case DeviceTransferEvent_BackupStart() when backupStart != null:
return backupStart(_that);case DeviceTransferEvent_BackupSucceed() when backupSucceed != null:
return backupSucceed(_that);case DeviceTransferEvent_BackupFailed() when backupFailed != null:
return backupFailed(_that);case DeviceTransferEvent_RestoreProgress() when restoreProgress != null:
return restoreProgress(_that);case DeviceTransferEvent_BackupProgress() when backupProgress != null:
return backupProgress(_that);case DeviceTransferEvent_RestoreNetworkSpeed() when restoreNetworkSpeed != null:
return restoreNetworkSpeed(_that);case DeviceTransferEvent_BackupNetworkSpeed() when backupNetworkSpeed != null:
return backupNetworkSpeed(_that);case DeviceTransferEvent_BackupRequestReceived() when backupRequestReceived != null:
return backupRequestReceived(_that);case DeviceTransferEvent_RestoreRequestReceived() when restoreRequestReceived != null:
return restoreRequestReceived(_that);case DeviceTransferEvent_ConnectionFailed() when connectionFailed != null:
return connectionFailed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( DeviceTransferEvent_RestoreConnected value)  restoreConnected,required TResult Function( DeviceTransferEvent_RestoreStart value)  restoreStart,required TResult Function( DeviceTransferEvent_RestoreSucceed value)  restoreSucceed,required TResult Function( DeviceTransferEvent_RestoreFailed value)  restoreFailed,required TResult Function( DeviceTransferEvent_BackupServerCreated value)  backupServerCreated,required TResult Function( DeviceTransferEvent_BackupStart value)  backupStart,required TResult Function( DeviceTransferEvent_BackupSucceed value)  backupSucceed,required TResult Function( DeviceTransferEvent_BackupFailed value)  backupFailed,required TResult Function( DeviceTransferEvent_RestoreProgress value)  restoreProgress,required TResult Function( DeviceTransferEvent_BackupProgress value)  backupProgress,required TResult Function( DeviceTransferEvent_RestoreNetworkSpeed value)  restoreNetworkSpeed,required TResult Function( DeviceTransferEvent_BackupNetworkSpeed value)  backupNetworkSpeed,required TResult Function( DeviceTransferEvent_BackupRequestReceived value)  backupRequestReceived,required TResult Function( DeviceTransferEvent_RestoreRequestReceived value)  restoreRequestReceived,required TResult Function( DeviceTransferEvent_ConnectionFailed value)  connectionFailed,}){
final _that = this;
switch (_that) {
case DeviceTransferEvent_RestoreConnected():
return restoreConnected(_that);case DeviceTransferEvent_RestoreStart():
return restoreStart(_that);case DeviceTransferEvent_RestoreSucceed():
return restoreSucceed(_that);case DeviceTransferEvent_RestoreFailed():
return restoreFailed(_that);case DeviceTransferEvent_BackupServerCreated():
return backupServerCreated(_that);case DeviceTransferEvent_BackupStart():
return backupStart(_that);case DeviceTransferEvent_BackupSucceed():
return backupSucceed(_that);case DeviceTransferEvent_BackupFailed():
return backupFailed(_that);case DeviceTransferEvent_RestoreProgress():
return restoreProgress(_that);case DeviceTransferEvent_BackupProgress():
return backupProgress(_that);case DeviceTransferEvent_RestoreNetworkSpeed():
return restoreNetworkSpeed(_that);case DeviceTransferEvent_BackupNetworkSpeed():
return backupNetworkSpeed(_that);case DeviceTransferEvent_BackupRequestReceived():
return backupRequestReceived(_that);case DeviceTransferEvent_RestoreRequestReceived():
return restoreRequestReceived(_that);case DeviceTransferEvent_ConnectionFailed():
return connectionFailed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( DeviceTransferEvent_RestoreConnected value)?  restoreConnected,TResult? Function( DeviceTransferEvent_RestoreStart value)?  restoreStart,TResult? Function( DeviceTransferEvent_RestoreSucceed value)?  restoreSucceed,TResult? Function( DeviceTransferEvent_RestoreFailed value)?  restoreFailed,TResult? Function( DeviceTransferEvent_BackupServerCreated value)?  backupServerCreated,TResult? Function( DeviceTransferEvent_BackupStart value)?  backupStart,TResult? Function( DeviceTransferEvent_BackupSucceed value)?  backupSucceed,TResult? Function( DeviceTransferEvent_BackupFailed value)?  backupFailed,TResult? Function( DeviceTransferEvent_RestoreProgress value)?  restoreProgress,TResult? Function( DeviceTransferEvent_BackupProgress value)?  backupProgress,TResult? Function( DeviceTransferEvent_RestoreNetworkSpeed value)?  restoreNetworkSpeed,TResult? Function( DeviceTransferEvent_BackupNetworkSpeed value)?  backupNetworkSpeed,TResult? Function( DeviceTransferEvent_BackupRequestReceived value)?  backupRequestReceived,TResult? Function( DeviceTransferEvent_RestoreRequestReceived value)?  restoreRequestReceived,TResult? Function( DeviceTransferEvent_ConnectionFailed value)?  connectionFailed,}){
final _that = this;
switch (_that) {
case DeviceTransferEvent_RestoreConnected() when restoreConnected != null:
return restoreConnected(_that);case DeviceTransferEvent_RestoreStart() when restoreStart != null:
return restoreStart(_that);case DeviceTransferEvent_RestoreSucceed() when restoreSucceed != null:
return restoreSucceed(_that);case DeviceTransferEvent_RestoreFailed() when restoreFailed != null:
return restoreFailed(_that);case DeviceTransferEvent_BackupServerCreated() when backupServerCreated != null:
return backupServerCreated(_that);case DeviceTransferEvent_BackupStart() when backupStart != null:
return backupStart(_that);case DeviceTransferEvent_BackupSucceed() when backupSucceed != null:
return backupSucceed(_that);case DeviceTransferEvent_BackupFailed() when backupFailed != null:
return backupFailed(_that);case DeviceTransferEvent_RestoreProgress() when restoreProgress != null:
return restoreProgress(_that);case DeviceTransferEvent_BackupProgress() when backupProgress != null:
return backupProgress(_that);case DeviceTransferEvent_RestoreNetworkSpeed() when restoreNetworkSpeed != null:
return restoreNetworkSpeed(_that);case DeviceTransferEvent_BackupNetworkSpeed() when backupNetworkSpeed != null:
return backupNetworkSpeed(_that);case DeviceTransferEvent_BackupRequestReceived() when backupRequestReceived != null:
return backupRequestReceived(_that);case DeviceTransferEvent_RestoreRequestReceived() when restoreRequestReceived != null:
return restoreRequestReceived(_that);case DeviceTransferEvent_ConnectionFailed() when connectionFailed != null:
return connectionFailed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  restoreConnected,TResult Function()?  restoreStart,TResult Function()?  restoreSucceed,TResult Function()?  restoreFailed,TResult Function()?  backupServerCreated,TResult Function()?  backupStart,TResult Function()?  backupSucceed,TResult Function()?  backupFailed,TResult Function( double field0)?  restoreProgress,TResult Function( double field0)?  backupProgress,TResult Function( double field0)?  restoreNetworkSpeed,TResult Function( double field0)?  backupNetworkSpeed,TResult Function()?  backupRequestReceived,TResult Function()?  restoreRequestReceived,TResult Function( ConnectionFailedReason field0)?  connectionFailed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case DeviceTransferEvent_RestoreConnected() when restoreConnected != null:
return restoreConnected();case DeviceTransferEvent_RestoreStart() when restoreStart != null:
return restoreStart();case DeviceTransferEvent_RestoreSucceed() when restoreSucceed != null:
return restoreSucceed();case DeviceTransferEvent_RestoreFailed() when restoreFailed != null:
return restoreFailed();case DeviceTransferEvent_BackupServerCreated() when backupServerCreated != null:
return backupServerCreated();case DeviceTransferEvent_BackupStart() when backupStart != null:
return backupStart();case DeviceTransferEvent_BackupSucceed() when backupSucceed != null:
return backupSucceed();case DeviceTransferEvent_BackupFailed() when backupFailed != null:
return backupFailed();case DeviceTransferEvent_RestoreProgress() when restoreProgress != null:
return restoreProgress(_that.field0);case DeviceTransferEvent_BackupProgress() when backupProgress != null:
return backupProgress(_that.field0);case DeviceTransferEvent_RestoreNetworkSpeed() when restoreNetworkSpeed != null:
return restoreNetworkSpeed(_that.field0);case DeviceTransferEvent_BackupNetworkSpeed() when backupNetworkSpeed != null:
return backupNetworkSpeed(_that.field0);case DeviceTransferEvent_BackupRequestReceived() when backupRequestReceived != null:
return backupRequestReceived();case DeviceTransferEvent_RestoreRequestReceived() when restoreRequestReceived != null:
return restoreRequestReceived();case DeviceTransferEvent_ConnectionFailed() when connectionFailed != null:
return connectionFailed(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  restoreConnected,required TResult Function()  restoreStart,required TResult Function()  restoreSucceed,required TResult Function()  restoreFailed,required TResult Function()  backupServerCreated,required TResult Function()  backupStart,required TResult Function()  backupSucceed,required TResult Function()  backupFailed,required TResult Function( double field0)  restoreProgress,required TResult Function( double field0)  backupProgress,required TResult Function( double field0)  restoreNetworkSpeed,required TResult Function( double field0)  backupNetworkSpeed,required TResult Function()  backupRequestReceived,required TResult Function()  restoreRequestReceived,required TResult Function( ConnectionFailedReason field0)  connectionFailed,}) {final _that = this;
switch (_that) {
case DeviceTransferEvent_RestoreConnected():
return restoreConnected();case DeviceTransferEvent_RestoreStart():
return restoreStart();case DeviceTransferEvent_RestoreSucceed():
return restoreSucceed();case DeviceTransferEvent_RestoreFailed():
return restoreFailed();case DeviceTransferEvent_BackupServerCreated():
return backupServerCreated();case DeviceTransferEvent_BackupStart():
return backupStart();case DeviceTransferEvent_BackupSucceed():
return backupSucceed();case DeviceTransferEvent_BackupFailed():
return backupFailed();case DeviceTransferEvent_RestoreProgress():
return restoreProgress(_that.field0);case DeviceTransferEvent_BackupProgress():
return backupProgress(_that.field0);case DeviceTransferEvent_RestoreNetworkSpeed():
return restoreNetworkSpeed(_that.field0);case DeviceTransferEvent_BackupNetworkSpeed():
return backupNetworkSpeed(_that.field0);case DeviceTransferEvent_BackupRequestReceived():
return backupRequestReceived();case DeviceTransferEvent_RestoreRequestReceived():
return restoreRequestReceived();case DeviceTransferEvent_ConnectionFailed():
return connectionFailed(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  restoreConnected,TResult? Function()?  restoreStart,TResult? Function()?  restoreSucceed,TResult? Function()?  restoreFailed,TResult? Function()?  backupServerCreated,TResult? Function()?  backupStart,TResult? Function()?  backupSucceed,TResult? Function()?  backupFailed,TResult? Function( double field0)?  restoreProgress,TResult? Function( double field0)?  backupProgress,TResult? Function( double field0)?  restoreNetworkSpeed,TResult? Function( double field0)?  backupNetworkSpeed,TResult? Function()?  backupRequestReceived,TResult? Function()?  restoreRequestReceived,TResult? Function( ConnectionFailedReason field0)?  connectionFailed,}) {final _that = this;
switch (_that) {
case DeviceTransferEvent_RestoreConnected() when restoreConnected != null:
return restoreConnected();case DeviceTransferEvent_RestoreStart() when restoreStart != null:
return restoreStart();case DeviceTransferEvent_RestoreSucceed() when restoreSucceed != null:
return restoreSucceed();case DeviceTransferEvent_RestoreFailed() when restoreFailed != null:
return restoreFailed();case DeviceTransferEvent_BackupServerCreated() when backupServerCreated != null:
return backupServerCreated();case DeviceTransferEvent_BackupStart() when backupStart != null:
return backupStart();case DeviceTransferEvent_BackupSucceed() when backupSucceed != null:
return backupSucceed();case DeviceTransferEvent_BackupFailed() when backupFailed != null:
return backupFailed();case DeviceTransferEvent_RestoreProgress() when restoreProgress != null:
return restoreProgress(_that.field0);case DeviceTransferEvent_BackupProgress() when backupProgress != null:
return backupProgress(_that.field0);case DeviceTransferEvent_RestoreNetworkSpeed() when restoreNetworkSpeed != null:
return restoreNetworkSpeed(_that.field0);case DeviceTransferEvent_BackupNetworkSpeed() when backupNetworkSpeed != null:
return backupNetworkSpeed(_that.field0);case DeviceTransferEvent_BackupRequestReceived() when backupRequestReceived != null:
return backupRequestReceived();case DeviceTransferEvent_RestoreRequestReceived() when restoreRequestReceived != null:
return restoreRequestReceived();case DeviceTransferEvent_ConnectionFailed() when connectionFailed != null:
return connectionFailed(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class DeviceTransferEvent_RestoreConnected extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreConnected(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreConnected);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.restoreConnected()';
}


}




/// @nodoc


class DeviceTransferEvent_RestoreStart extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreStart(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreStart);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.restoreStart()';
}


}




/// @nodoc


class DeviceTransferEvent_RestoreSucceed extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreSucceed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreSucceed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.restoreSucceed()';
}


}




/// @nodoc


class DeviceTransferEvent_RestoreFailed extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreFailed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.restoreFailed()';
}


}




/// @nodoc


class DeviceTransferEvent_BackupServerCreated extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupServerCreated(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupServerCreated);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.backupServerCreated()';
}


}




/// @nodoc


class DeviceTransferEvent_BackupStart extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupStart(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupStart);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.backupStart()';
}


}




/// @nodoc


class DeviceTransferEvent_BackupSucceed extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupSucceed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupSucceed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.backupSucceed()';
}


}




/// @nodoc


class DeviceTransferEvent_BackupFailed extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupFailed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.backupFailed()';
}


}




/// @nodoc


class DeviceTransferEvent_RestoreProgress extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreProgress(this.field0): super._();
  

 final  double field0;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DeviceTransferEvent_RestoreProgressCopyWith<DeviceTransferEvent_RestoreProgress> get copyWith => _$DeviceTransferEvent_RestoreProgressCopyWithImpl<DeviceTransferEvent_RestoreProgress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreProgress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DeviceTransferEvent.restoreProgress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DeviceTransferEvent_RestoreProgressCopyWith<$Res> implements $DeviceTransferEventCopyWith<$Res> {
  factory $DeviceTransferEvent_RestoreProgressCopyWith(DeviceTransferEvent_RestoreProgress value, $Res Function(DeviceTransferEvent_RestoreProgress) _then) = _$DeviceTransferEvent_RestoreProgressCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$DeviceTransferEvent_RestoreProgressCopyWithImpl<$Res>
    implements $DeviceTransferEvent_RestoreProgressCopyWith<$Res> {
  _$DeviceTransferEvent_RestoreProgressCopyWithImpl(this._self, this._then);

  final DeviceTransferEvent_RestoreProgress _self;
  final $Res Function(DeviceTransferEvent_RestoreProgress) _then;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DeviceTransferEvent_RestoreProgress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class DeviceTransferEvent_BackupProgress extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupProgress(this.field0): super._();
  

 final  double field0;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DeviceTransferEvent_BackupProgressCopyWith<DeviceTransferEvent_BackupProgress> get copyWith => _$DeviceTransferEvent_BackupProgressCopyWithImpl<DeviceTransferEvent_BackupProgress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupProgress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DeviceTransferEvent.backupProgress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DeviceTransferEvent_BackupProgressCopyWith<$Res> implements $DeviceTransferEventCopyWith<$Res> {
  factory $DeviceTransferEvent_BackupProgressCopyWith(DeviceTransferEvent_BackupProgress value, $Res Function(DeviceTransferEvent_BackupProgress) _then) = _$DeviceTransferEvent_BackupProgressCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$DeviceTransferEvent_BackupProgressCopyWithImpl<$Res>
    implements $DeviceTransferEvent_BackupProgressCopyWith<$Res> {
  _$DeviceTransferEvent_BackupProgressCopyWithImpl(this._self, this._then);

  final DeviceTransferEvent_BackupProgress _self;
  final $Res Function(DeviceTransferEvent_BackupProgress) _then;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DeviceTransferEvent_BackupProgress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class DeviceTransferEvent_RestoreNetworkSpeed extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreNetworkSpeed(this.field0): super._();
  

 final  double field0;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DeviceTransferEvent_RestoreNetworkSpeedCopyWith<DeviceTransferEvent_RestoreNetworkSpeed> get copyWith => _$DeviceTransferEvent_RestoreNetworkSpeedCopyWithImpl<DeviceTransferEvent_RestoreNetworkSpeed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreNetworkSpeed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DeviceTransferEvent.restoreNetworkSpeed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DeviceTransferEvent_RestoreNetworkSpeedCopyWith<$Res> implements $DeviceTransferEventCopyWith<$Res> {
  factory $DeviceTransferEvent_RestoreNetworkSpeedCopyWith(DeviceTransferEvent_RestoreNetworkSpeed value, $Res Function(DeviceTransferEvent_RestoreNetworkSpeed) _then) = _$DeviceTransferEvent_RestoreNetworkSpeedCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$DeviceTransferEvent_RestoreNetworkSpeedCopyWithImpl<$Res>
    implements $DeviceTransferEvent_RestoreNetworkSpeedCopyWith<$Res> {
  _$DeviceTransferEvent_RestoreNetworkSpeedCopyWithImpl(this._self, this._then);

  final DeviceTransferEvent_RestoreNetworkSpeed _self;
  final $Res Function(DeviceTransferEvent_RestoreNetworkSpeed) _then;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DeviceTransferEvent_RestoreNetworkSpeed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class DeviceTransferEvent_BackupNetworkSpeed extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupNetworkSpeed(this.field0): super._();
  

 final  double field0;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DeviceTransferEvent_BackupNetworkSpeedCopyWith<DeviceTransferEvent_BackupNetworkSpeed> get copyWith => _$DeviceTransferEvent_BackupNetworkSpeedCopyWithImpl<DeviceTransferEvent_BackupNetworkSpeed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupNetworkSpeed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DeviceTransferEvent.backupNetworkSpeed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DeviceTransferEvent_BackupNetworkSpeedCopyWith<$Res> implements $DeviceTransferEventCopyWith<$Res> {
  factory $DeviceTransferEvent_BackupNetworkSpeedCopyWith(DeviceTransferEvent_BackupNetworkSpeed value, $Res Function(DeviceTransferEvent_BackupNetworkSpeed) _then) = _$DeviceTransferEvent_BackupNetworkSpeedCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$DeviceTransferEvent_BackupNetworkSpeedCopyWithImpl<$Res>
    implements $DeviceTransferEvent_BackupNetworkSpeedCopyWith<$Res> {
  _$DeviceTransferEvent_BackupNetworkSpeedCopyWithImpl(this._self, this._then);

  final DeviceTransferEvent_BackupNetworkSpeed _self;
  final $Res Function(DeviceTransferEvent_BackupNetworkSpeed) _then;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DeviceTransferEvent_BackupNetworkSpeed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class DeviceTransferEvent_BackupRequestReceived extends DeviceTransferEvent {
  const DeviceTransferEvent_BackupRequestReceived(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_BackupRequestReceived);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.backupRequestReceived()';
}


}




/// @nodoc


class DeviceTransferEvent_RestoreRequestReceived extends DeviceTransferEvent {
  const DeviceTransferEvent_RestoreRequestReceived(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_RestoreRequestReceived);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'DeviceTransferEvent.restoreRequestReceived()';
}


}




/// @nodoc


class DeviceTransferEvent_ConnectionFailed extends DeviceTransferEvent {
  const DeviceTransferEvent_ConnectionFailed(this.field0): super._();
  

 final  ConnectionFailedReason field0;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DeviceTransferEvent_ConnectionFailedCopyWith<DeviceTransferEvent_ConnectionFailed> get copyWith => _$DeviceTransferEvent_ConnectionFailedCopyWithImpl<DeviceTransferEvent_ConnectionFailed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DeviceTransferEvent_ConnectionFailed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DeviceTransferEvent.connectionFailed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DeviceTransferEvent_ConnectionFailedCopyWith<$Res> implements $DeviceTransferEventCopyWith<$Res> {
  factory $DeviceTransferEvent_ConnectionFailedCopyWith(DeviceTransferEvent_ConnectionFailed value, $Res Function(DeviceTransferEvent_ConnectionFailed) _then) = _$DeviceTransferEvent_ConnectionFailedCopyWithImpl;
@useResult
$Res call({
 ConnectionFailedReason field0
});




}
/// @nodoc
class _$DeviceTransferEvent_ConnectionFailedCopyWithImpl<$Res>
    implements $DeviceTransferEvent_ConnectionFailedCopyWith<$Res> {
  _$DeviceTransferEvent_ConnectionFailedCopyWithImpl(this._self, this._then);

  final DeviceTransferEvent_ConnectionFailed _self;
  final $Res Function(DeviceTransferEvent_ConnectionFailed) _then;

/// Create a copy of DeviceTransferEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DeviceTransferEvent_ConnectionFailed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ConnectionFailedReason,
  ));
}


}

// dart format on
