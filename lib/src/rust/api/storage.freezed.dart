// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'storage.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CreateMessageWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateMessageWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'CreateMessageWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $CreateMessageWithStorageOutcomeCopyWith<$Res>  {
$CreateMessageWithStorageOutcomeCopyWith(CreateMessageWithStorageOutcome _, $Res Function(CreateMessageWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [CreateMessageWithStorageOutcome].
extension CreateMessageWithStorageOutcomePatterns on CreateMessageWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CreateMessageWithStorageOutcome_Success value)?  success,TResult Function( CreateMessageWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CreateMessageWithStorageOutcome_Success() when success != null:
return success(_that);case CreateMessageWithStorageOutcome_Failure() when failure != null:
return failure(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CreateMessageWithStorageOutcome_Success value)  success,required TResult Function( CreateMessageWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case CreateMessageWithStorageOutcome_Success():
return success(_that);case CreateMessageWithStorageOutcome_Failure():
return failure(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CreateMessageWithStorageOutcome_Success value)?  success,TResult? Function( CreateMessageWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case CreateMessageWithStorageOutcome_Success() when success != null:
return success(_that);case CreateMessageWithStorageOutcome_Failure() when failure != null:
return failure(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( CreateMessageWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CreateMessageWithStorageOutcome_Success() when success != null:
return success(_that.field0);case CreateMessageWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( CreateMessageWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case CreateMessageWithStorageOutcome_Success():
return success(_that.field0);case CreateMessageWithStorageOutcome_Failure():
return failure(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( CreateMessageWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case CreateMessageWithStorageOutcome_Success() when success != null:
return success(_that.field0);case CreateMessageWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class CreateMessageWithStorageOutcome_Success extends CreateMessageWithStorageOutcome {
  const CreateMessageWithStorageOutcome_Success(this.field0): super._();


@override final  CreateMessageWithStorageResult field0;

/// Create a copy of CreateMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CreateMessageWithStorageOutcome_SuccessCopyWith<CreateMessageWithStorageOutcome_Success> get copyWith => _$CreateMessageWithStorageOutcome_SuccessCopyWithImpl<CreateMessageWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateMessageWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'CreateMessageWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CreateMessageWithStorageOutcome_SuccessCopyWith<$Res> implements $CreateMessageWithStorageOutcomeCopyWith<$Res> {
  factory $CreateMessageWithStorageOutcome_SuccessCopyWith(CreateMessageWithStorageOutcome_Success value, $Res Function(CreateMessageWithStorageOutcome_Success) _then) = _$CreateMessageWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 CreateMessageWithStorageResult field0
});




}
/// @nodoc
class _$CreateMessageWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $CreateMessageWithStorageOutcome_SuccessCopyWith<$Res> {
  _$CreateMessageWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final CreateMessageWithStorageOutcome_Success _self;
  final $Res Function(CreateMessageWithStorageOutcome_Success) _then;

/// Create a copy of CreateMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CreateMessageWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as CreateMessageWithStorageResult,
  ));
}


}

/// @nodoc


class CreateMessageWithStorageOutcome_Failure extends CreateMessageWithStorageOutcome {
  const CreateMessageWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of CreateMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CreateMessageWithStorageOutcome_FailureCopyWith<CreateMessageWithStorageOutcome_Failure> get copyWith => _$CreateMessageWithStorageOutcome_FailureCopyWithImpl<CreateMessageWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateMessageWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'CreateMessageWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CreateMessageWithStorageOutcome_FailureCopyWith<$Res> implements $CreateMessageWithStorageOutcomeCopyWith<$Res> {
  factory $CreateMessageWithStorageOutcome_FailureCopyWith(CreateMessageWithStorageOutcome_Failure value, $Res Function(CreateMessageWithStorageOutcome_Failure) _then) = _$CreateMessageWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$CreateMessageWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $CreateMessageWithStorageOutcome_FailureCopyWith<$Res> {
  _$CreateMessageWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final CreateMessageWithStorageOutcome_Failure _self;
  final $Res Function(CreateMessageWithStorageOutcome_Failure) _then;

/// Create a copy of CreateMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CreateMessageWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

// dart format on
