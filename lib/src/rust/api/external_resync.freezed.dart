// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'external_resync.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$AcceptExternalCommitOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AcceptExternalCommitOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'AcceptExternalCommitOutcome(field0: $field0)';
}


}

/// @nodoc
class $AcceptExternalCommitOutcomeCopyWith<$Res>  {
$AcceptExternalCommitOutcomeCopyWith(AcceptExternalCommitOutcome _, $Res Function(AcceptExternalCommitOutcome) __);
}


/// Adds pattern-matching-related methods to [AcceptExternalCommitOutcome].
extension AcceptExternalCommitOutcomePatterns on AcceptExternalCommitOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AcceptExternalCommitOutcome_Success value)?  success,TResult Function( AcceptExternalCommitOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AcceptExternalCommitOutcome_Success() when success != null:
return success(_that);case AcceptExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AcceptExternalCommitOutcome_Success value)  success,required TResult Function( AcceptExternalCommitOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case AcceptExternalCommitOutcome_Success():
return success(_that);case AcceptExternalCommitOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AcceptExternalCommitOutcome_Success value)?  success,TResult? Function( AcceptExternalCommitOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case AcceptExternalCommitOutcome_Success() when success != null:
return success(_that);case AcceptExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( AcceptExternalCommitResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AcceptExternalCommitOutcome_Success() when success != null:
return success(_that.field0);case AcceptExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( AcceptExternalCommitResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case AcceptExternalCommitOutcome_Success():
return success(_that.field0);case AcceptExternalCommitOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( AcceptExternalCommitResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case AcceptExternalCommitOutcome_Success() when success != null:
return success(_that.field0);case AcceptExternalCommitOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class AcceptExternalCommitOutcome_Success extends AcceptExternalCommitOutcome {
  const AcceptExternalCommitOutcome_Success(this.field0): super._();


@override final  AcceptExternalCommitResult field0;

/// Create a copy of AcceptExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AcceptExternalCommitOutcome_SuccessCopyWith<AcceptExternalCommitOutcome_Success> get copyWith => _$AcceptExternalCommitOutcome_SuccessCopyWithImpl<AcceptExternalCommitOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AcceptExternalCommitOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AcceptExternalCommitOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AcceptExternalCommitOutcome_SuccessCopyWith<$Res> implements $AcceptExternalCommitOutcomeCopyWith<$Res> {
  factory $AcceptExternalCommitOutcome_SuccessCopyWith(AcceptExternalCommitOutcome_Success value, $Res Function(AcceptExternalCommitOutcome_Success) _then) = _$AcceptExternalCommitOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 AcceptExternalCommitResult field0
});




}
/// @nodoc
class _$AcceptExternalCommitOutcome_SuccessCopyWithImpl<$Res>
    implements $AcceptExternalCommitOutcome_SuccessCopyWith<$Res> {
  _$AcceptExternalCommitOutcome_SuccessCopyWithImpl(this._self, this._then);

  final AcceptExternalCommitOutcome_Success _self;
  final $Res Function(AcceptExternalCommitOutcome_Success) _then;

/// Create a copy of AcceptExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AcceptExternalCommitOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as AcceptExternalCommitResult,
  ));
}


}

/// @nodoc


class AcceptExternalCommitOutcome_Failure extends AcceptExternalCommitOutcome {
  const AcceptExternalCommitOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of AcceptExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AcceptExternalCommitOutcome_FailureCopyWith<AcceptExternalCommitOutcome_Failure> get copyWith => _$AcceptExternalCommitOutcome_FailureCopyWithImpl<AcceptExternalCommitOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AcceptExternalCommitOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AcceptExternalCommitOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AcceptExternalCommitOutcome_FailureCopyWith<$Res> implements $AcceptExternalCommitOutcomeCopyWith<$Res> {
  factory $AcceptExternalCommitOutcome_FailureCopyWith(AcceptExternalCommitOutcome_Failure value, $Res Function(AcceptExternalCommitOutcome_Failure) _then) = _$AcceptExternalCommitOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$AcceptExternalCommitOutcome_FailureCopyWithImpl<$Res>
    implements $AcceptExternalCommitOutcome_FailureCopyWith<$Res> {
  _$AcceptExternalCommitOutcome_FailureCopyWithImpl(this._self, this._then);

  final AcceptExternalCommitOutcome_Failure _self;
  final $Res Function(AcceptExternalCommitOutcome_Failure) _then;

/// Create a copy of AcceptExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AcceptExternalCommitOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$InspectExternalCommitCandidateOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is InspectExternalCommitCandidateOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'InspectExternalCommitCandidateOutcome(field0: $field0)';
}


}

/// @nodoc
class $InspectExternalCommitCandidateOutcomeCopyWith<$Res>  {
$InspectExternalCommitCandidateOutcomeCopyWith(InspectExternalCommitCandidateOutcome _, $Res Function(InspectExternalCommitCandidateOutcome) __);
}


/// Adds pattern-matching-related methods to [InspectExternalCommitCandidateOutcome].
extension InspectExternalCommitCandidateOutcomePatterns on InspectExternalCommitCandidateOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( InspectExternalCommitCandidateOutcome_Success value)?  success,TResult Function( InspectExternalCommitCandidateOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case InspectExternalCommitCandidateOutcome_Success() when success != null:
return success(_that);case InspectExternalCommitCandidateOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( InspectExternalCommitCandidateOutcome_Success value)  success,required TResult Function( InspectExternalCommitCandidateOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case InspectExternalCommitCandidateOutcome_Success():
return success(_that);case InspectExternalCommitCandidateOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( InspectExternalCommitCandidateOutcome_Success value)?  success,TResult? Function( InspectExternalCommitCandidateOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case InspectExternalCommitCandidateOutcome_Success() when success != null:
return success(_that);case InspectExternalCommitCandidateOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( ExternalCommitCandidateInfo field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case InspectExternalCommitCandidateOutcome_Success() when success != null:
return success(_that.field0);case InspectExternalCommitCandidateOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( ExternalCommitCandidateInfo field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case InspectExternalCommitCandidateOutcome_Success():
return success(_that.field0);case InspectExternalCommitCandidateOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( ExternalCommitCandidateInfo field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case InspectExternalCommitCandidateOutcome_Success() when success != null:
return success(_that.field0);case InspectExternalCommitCandidateOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class InspectExternalCommitCandidateOutcome_Success extends InspectExternalCommitCandidateOutcome {
  const InspectExternalCommitCandidateOutcome_Success(this.field0): super._();


@override final  ExternalCommitCandidateInfo field0;

/// Create a copy of InspectExternalCommitCandidateOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$InspectExternalCommitCandidateOutcome_SuccessCopyWith<InspectExternalCommitCandidateOutcome_Success> get copyWith => _$InspectExternalCommitCandidateOutcome_SuccessCopyWithImpl<InspectExternalCommitCandidateOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is InspectExternalCommitCandidateOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'InspectExternalCommitCandidateOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $InspectExternalCommitCandidateOutcome_SuccessCopyWith<$Res> implements $InspectExternalCommitCandidateOutcomeCopyWith<$Res> {
  factory $InspectExternalCommitCandidateOutcome_SuccessCopyWith(InspectExternalCommitCandidateOutcome_Success value, $Res Function(InspectExternalCommitCandidateOutcome_Success) _then) = _$InspectExternalCommitCandidateOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 ExternalCommitCandidateInfo field0
});




}
/// @nodoc
class _$InspectExternalCommitCandidateOutcome_SuccessCopyWithImpl<$Res>
    implements $InspectExternalCommitCandidateOutcome_SuccessCopyWith<$Res> {
  _$InspectExternalCommitCandidateOutcome_SuccessCopyWithImpl(this._self, this._then);

  final InspectExternalCommitCandidateOutcome_Success _self;
  final $Res Function(InspectExternalCommitCandidateOutcome_Success) _then;

/// Create a copy of InspectExternalCommitCandidateOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(InspectExternalCommitCandidateOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ExternalCommitCandidateInfo,
  ));
}


}

/// @nodoc


class InspectExternalCommitCandidateOutcome_Failure extends InspectExternalCommitCandidateOutcome {
  const InspectExternalCommitCandidateOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of InspectExternalCommitCandidateOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$InspectExternalCommitCandidateOutcome_FailureCopyWith<InspectExternalCommitCandidateOutcome_Failure> get copyWith => _$InspectExternalCommitCandidateOutcome_FailureCopyWithImpl<InspectExternalCommitCandidateOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is InspectExternalCommitCandidateOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'InspectExternalCommitCandidateOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $InspectExternalCommitCandidateOutcome_FailureCopyWith<$Res> implements $InspectExternalCommitCandidateOutcomeCopyWith<$Res> {
  factory $InspectExternalCommitCandidateOutcome_FailureCopyWith(InspectExternalCommitCandidateOutcome_Failure value, $Res Function(InspectExternalCommitCandidateOutcome_Failure) _then) = _$InspectExternalCommitCandidateOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$InspectExternalCommitCandidateOutcome_FailureCopyWithImpl<$Res>
    implements $InspectExternalCommitCandidateOutcome_FailureCopyWith<$Res> {
  _$InspectExternalCommitCandidateOutcome_FailureCopyWithImpl(this._self, this._then);

  final InspectExternalCommitCandidateOutcome_Failure _self;
  final $Res Function(InspectExternalCommitCandidateOutcome_Failure) _then;

/// Create a copy of InspectExternalCommitCandidateOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(InspectExternalCommitCandidateOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$PrepareExternalCommitOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PrepareExternalCommitOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'PrepareExternalCommitOutcome(field0: $field0)';
}


}

/// @nodoc
class $PrepareExternalCommitOutcomeCopyWith<$Res>  {
$PrepareExternalCommitOutcomeCopyWith(PrepareExternalCommitOutcome _, $Res Function(PrepareExternalCommitOutcome) __);
}


/// Adds pattern-matching-related methods to [PrepareExternalCommitOutcome].
extension PrepareExternalCommitOutcomePatterns on PrepareExternalCommitOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( PrepareExternalCommitOutcome_Success value)?  success,TResult Function( PrepareExternalCommitOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case PrepareExternalCommitOutcome_Success() when success != null:
return success(_that);case PrepareExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( PrepareExternalCommitOutcome_Success value)  success,required TResult Function( PrepareExternalCommitOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case PrepareExternalCommitOutcome_Success():
return success(_that);case PrepareExternalCommitOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( PrepareExternalCommitOutcome_Success value)?  success,TResult? Function( PrepareExternalCommitOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case PrepareExternalCommitOutcome_Success() when success != null:
return success(_that);case PrepareExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PrepareExternalCommitResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case PrepareExternalCommitOutcome_Success() when success != null:
return success(_that.field0);case PrepareExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PrepareExternalCommitResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case PrepareExternalCommitOutcome_Success():
return success(_that.field0);case PrepareExternalCommitOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PrepareExternalCommitResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case PrepareExternalCommitOutcome_Success() when success != null:
return success(_that.field0);case PrepareExternalCommitOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class PrepareExternalCommitOutcome_Success extends PrepareExternalCommitOutcome {
  const PrepareExternalCommitOutcome_Success(this.field0): super._();


@override final  PrepareExternalCommitResult field0;

/// Create a copy of PrepareExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PrepareExternalCommitOutcome_SuccessCopyWith<PrepareExternalCommitOutcome_Success> get copyWith => _$PrepareExternalCommitOutcome_SuccessCopyWithImpl<PrepareExternalCommitOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PrepareExternalCommitOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'PrepareExternalCommitOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PrepareExternalCommitOutcome_SuccessCopyWith<$Res> implements $PrepareExternalCommitOutcomeCopyWith<$Res> {
  factory $PrepareExternalCommitOutcome_SuccessCopyWith(PrepareExternalCommitOutcome_Success value, $Res Function(PrepareExternalCommitOutcome_Success) _then) = _$PrepareExternalCommitOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 PrepareExternalCommitResult field0
});




}
/// @nodoc
class _$PrepareExternalCommitOutcome_SuccessCopyWithImpl<$Res>
    implements $PrepareExternalCommitOutcome_SuccessCopyWith<$Res> {
  _$PrepareExternalCommitOutcome_SuccessCopyWithImpl(this._self, this._then);

  final PrepareExternalCommitOutcome_Success _self;
  final $Res Function(PrepareExternalCommitOutcome_Success) _then;

/// Create a copy of PrepareExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PrepareExternalCommitOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PrepareExternalCommitResult,
  ));
}


}

/// @nodoc


class PrepareExternalCommitOutcome_Failure extends PrepareExternalCommitOutcome {
  const PrepareExternalCommitOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of PrepareExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PrepareExternalCommitOutcome_FailureCopyWith<PrepareExternalCommitOutcome_Failure> get copyWith => _$PrepareExternalCommitOutcome_FailureCopyWithImpl<PrepareExternalCommitOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PrepareExternalCommitOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'PrepareExternalCommitOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PrepareExternalCommitOutcome_FailureCopyWith<$Res> implements $PrepareExternalCommitOutcomeCopyWith<$Res> {
  factory $PrepareExternalCommitOutcome_FailureCopyWith(PrepareExternalCommitOutcome_Failure value, $Res Function(PrepareExternalCommitOutcome_Failure) _then) = _$PrepareExternalCommitOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$PrepareExternalCommitOutcome_FailureCopyWithImpl<$Res>
    implements $PrepareExternalCommitOutcome_FailureCopyWith<$Res> {
  _$PrepareExternalCommitOutcome_FailureCopyWithImpl(this._self, this._then);

  final PrepareExternalCommitOutcome_Failure _self;
  final $Res Function(PrepareExternalCommitOutcome_Failure) _then;

/// Create a copy of PrepareExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PrepareExternalCommitOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$ProcessExternalCommitOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProcessExternalCommitOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'ProcessExternalCommitOutcome(field0: $field0)';
}


}

/// @nodoc
class $ProcessExternalCommitOutcomeCopyWith<$Res>  {
$ProcessExternalCommitOutcomeCopyWith(ProcessExternalCommitOutcome _, $Res Function(ProcessExternalCommitOutcome) __);
}


/// Adds pattern-matching-related methods to [ProcessExternalCommitOutcome].
extension ProcessExternalCommitOutcomePatterns on ProcessExternalCommitOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ProcessExternalCommitOutcome_Success value)?  success,TResult Function( ProcessExternalCommitOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ProcessExternalCommitOutcome_Success() when success != null:
return success(_that);case ProcessExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ProcessExternalCommitOutcome_Success value)  success,required TResult Function( ProcessExternalCommitOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case ProcessExternalCommitOutcome_Success():
return success(_that);case ProcessExternalCommitOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ProcessExternalCommitOutcome_Success value)?  success,TResult? Function( ProcessExternalCommitOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case ProcessExternalCommitOutcome_Success() when success != null:
return success(_that);case ProcessExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( ProcessExternalCommitResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ProcessExternalCommitOutcome_Success() when success != null:
return success(_that.field0);case ProcessExternalCommitOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( ProcessExternalCommitResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case ProcessExternalCommitOutcome_Success():
return success(_that.field0);case ProcessExternalCommitOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( ProcessExternalCommitResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case ProcessExternalCommitOutcome_Success() when success != null:
return success(_that.field0);case ProcessExternalCommitOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class ProcessExternalCommitOutcome_Success extends ProcessExternalCommitOutcome {
  const ProcessExternalCommitOutcome_Success(this.field0): super._();


@override final  ProcessExternalCommitResult field0;

/// Create a copy of ProcessExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProcessExternalCommitOutcome_SuccessCopyWith<ProcessExternalCommitOutcome_Success> get copyWith => _$ProcessExternalCommitOutcome_SuccessCopyWithImpl<ProcessExternalCommitOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProcessExternalCommitOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ProcessExternalCommitOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ProcessExternalCommitOutcome_SuccessCopyWith<$Res> implements $ProcessExternalCommitOutcomeCopyWith<$Res> {
  factory $ProcessExternalCommitOutcome_SuccessCopyWith(ProcessExternalCommitOutcome_Success value, $Res Function(ProcessExternalCommitOutcome_Success) _then) = _$ProcessExternalCommitOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 ProcessExternalCommitResult field0
});




}
/// @nodoc
class _$ProcessExternalCommitOutcome_SuccessCopyWithImpl<$Res>
    implements $ProcessExternalCommitOutcome_SuccessCopyWith<$Res> {
  _$ProcessExternalCommitOutcome_SuccessCopyWithImpl(this._self, this._then);

  final ProcessExternalCommitOutcome_Success _self;
  final $Res Function(ProcessExternalCommitOutcome_Success) _then;

/// Create a copy of ProcessExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ProcessExternalCommitOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ProcessExternalCommitResult,
  ));
}


}

/// @nodoc


class ProcessExternalCommitOutcome_Failure extends ProcessExternalCommitOutcome {
  const ProcessExternalCommitOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of ProcessExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProcessExternalCommitOutcome_FailureCopyWith<ProcessExternalCommitOutcome_Failure> get copyWith => _$ProcessExternalCommitOutcome_FailureCopyWithImpl<ProcessExternalCommitOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProcessExternalCommitOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ProcessExternalCommitOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ProcessExternalCommitOutcome_FailureCopyWith<$Res> implements $ProcessExternalCommitOutcomeCopyWith<$Res> {
  factory $ProcessExternalCommitOutcome_FailureCopyWith(ProcessExternalCommitOutcome_Failure value, $Res Function(ProcessExternalCommitOutcome_Failure) _then) = _$ProcessExternalCommitOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$ProcessExternalCommitOutcome_FailureCopyWithImpl<$Res>
    implements $ProcessExternalCommitOutcome_FailureCopyWith<$Res> {
  _$ProcessExternalCommitOutcome_FailureCopyWithImpl(this._self, this._then);

  final ProcessExternalCommitOutcome_Failure _self;
  final $Res Function(ProcessExternalCommitOutcome_Failure) _then;

/// Create a copy of ProcessExternalCommitOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ProcessExternalCommitOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

// dart format on
