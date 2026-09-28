// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'group_e2ee.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$AddMembersWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AddMembersWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'AddMembersWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $AddMembersWithStorageOutcomeCopyWith<$Res>  {
$AddMembersWithStorageOutcomeCopyWith(AddMembersWithStorageOutcome _, $Res Function(AddMembersWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [AddMembersWithStorageOutcome].
extension AddMembersWithStorageOutcomePatterns on AddMembersWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AddMembersWithStorageOutcome_Success value)?  success,TResult Function( AddMembersWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AddMembersWithStorageOutcome_Success() when success != null:
return success(_that);case AddMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AddMembersWithStorageOutcome_Success value)  success,required TResult Function( AddMembersWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case AddMembersWithStorageOutcome_Success():
return success(_that);case AddMembersWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AddMembersWithStorageOutcome_Success value)?  success,TResult? Function( AddMembersWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case AddMembersWithStorageOutcome_Success() when success != null:
return success(_that);case AddMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PendingCommitWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AddMembersWithStorageOutcome_Success() when success != null:
return success(_that.field0);case AddMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PendingCommitWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case AddMembersWithStorageOutcome_Success():
return success(_that.field0);case AddMembersWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PendingCommitWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case AddMembersWithStorageOutcome_Success() when success != null:
return success(_that.field0);case AddMembersWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class AddMembersWithStorageOutcome_Success extends AddMembersWithStorageOutcome {
  const AddMembersWithStorageOutcome_Success(this.field0): super._();


@override final  PendingCommitWithStorageResult field0;

/// Create a copy of AddMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AddMembersWithStorageOutcome_SuccessCopyWith<AddMembersWithStorageOutcome_Success> get copyWith => _$AddMembersWithStorageOutcome_SuccessCopyWithImpl<AddMembersWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AddMembersWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AddMembersWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AddMembersWithStorageOutcome_SuccessCopyWith<$Res> implements $AddMembersWithStorageOutcomeCopyWith<$Res> {
  factory $AddMembersWithStorageOutcome_SuccessCopyWith(AddMembersWithStorageOutcome_Success value, $Res Function(AddMembersWithStorageOutcome_Success) _then) = _$AddMembersWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 PendingCommitWithStorageResult field0
});




}
/// @nodoc
class _$AddMembersWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $AddMembersWithStorageOutcome_SuccessCopyWith<$Res> {
  _$AddMembersWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final AddMembersWithStorageOutcome_Success _self;
  final $Res Function(AddMembersWithStorageOutcome_Success) _then;

/// Create a copy of AddMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AddMembersWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PendingCommitWithStorageResult,
  ));
}


}

/// @nodoc


class AddMembersWithStorageOutcome_Failure extends AddMembersWithStorageOutcome {
  const AddMembersWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of AddMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AddMembersWithStorageOutcome_FailureCopyWith<AddMembersWithStorageOutcome_Failure> get copyWith => _$AddMembersWithStorageOutcome_FailureCopyWithImpl<AddMembersWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AddMembersWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AddMembersWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AddMembersWithStorageOutcome_FailureCopyWith<$Res> implements $AddMembersWithStorageOutcomeCopyWith<$Res> {
  factory $AddMembersWithStorageOutcome_FailureCopyWith(AddMembersWithStorageOutcome_Failure value, $Res Function(AddMembersWithStorageOutcome_Failure) _then) = _$AddMembersWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$AddMembersWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $AddMembersWithStorageOutcome_FailureCopyWith<$Res> {
  _$AddMembersWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final AddMembersWithStorageOutcome_Failure _self;
  final $Res Function(AddMembersWithStorageOutcome_Failure) _then;

/// Create a copy of AddMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AddMembersWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$CreateGroupWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateGroupWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'CreateGroupWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $CreateGroupWithStorageOutcomeCopyWith<$Res>  {
$CreateGroupWithStorageOutcomeCopyWith(CreateGroupWithStorageOutcome _, $Res Function(CreateGroupWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [CreateGroupWithStorageOutcome].
extension CreateGroupWithStorageOutcomePatterns on CreateGroupWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CreateGroupWithStorageOutcome_Success value)?  success,TResult Function( CreateGroupWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CreateGroupWithStorageOutcome_Success() when success != null:
return success(_that);case CreateGroupWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CreateGroupWithStorageOutcome_Success value)  success,required TResult Function( CreateGroupWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case CreateGroupWithStorageOutcome_Success():
return success(_that);case CreateGroupWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CreateGroupWithStorageOutcome_Success value)?  success,TResult? Function( CreateGroupWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case CreateGroupWithStorageOutcome_Success() when success != null:
return success(_that);case CreateGroupWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( CreateGroupWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CreateGroupWithStorageOutcome_Success() when success != null:
return success(_that.field0);case CreateGroupWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( CreateGroupWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case CreateGroupWithStorageOutcome_Success():
return success(_that.field0);case CreateGroupWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( CreateGroupWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case CreateGroupWithStorageOutcome_Success() when success != null:
return success(_that.field0);case CreateGroupWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class CreateGroupWithStorageOutcome_Success extends CreateGroupWithStorageOutcome {
  const CreateGroupWithStorageOutcome_Success(this.field0): super._();


@override final  CreateGroupWithStorageResult field0;

/// Create a copy of CreateGroupWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CreateGroupWithStorageOutcome_SuccessCopyWith<CreateGroupWithStorageOutcome_Success> get copyWith => _$CreateGroupWithStorageOutcome_SuccessCopyWithImpl<CreateGroupWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateGroupWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'CreateGroupWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CreateGroupWithStorageOutcome_SuccessCopyWith<$Res> implements $CreateGroupWithStorageOutcomeCopyWith<$Res> {
  factory $CreateGroupWithStorageOutcome_SuccessCopyWith(CreateGroupWithStorageOutcome_Success value, $Res Function(CreateGroupWithStorageOutcome_Success) _then) = _$CreateGroupWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 CreateGroupWithStorageResult field0
});




}
/// @nodoc
class _$CreateGroupWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $CreateGroupWithStorageOutcome_SuccessCopyWith<$Res> {
  _$CreateGroupWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final CreateGroupWithStorageOutcome_Success _self;
  final $Res Function(CreateGroupWithStorageOutcome_Success) _then;

/// Create a copy of CreateGroupWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CreateGroupWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as CreateGroupWithStorageResult,
  ));
}


}

/// @nodoc


class CreateGroupWithStorageOutcome_Failure extends CreateGroupWithStorageOutcome {
  const CreateGroupWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of CreateGroupWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CreateGroupWithStorageOutcome_FailureCopyWith<CreateGroupWithStorageOutcome_Failure> get copyWith => _$CreateGroupWithStorageOutcome_FailureCopyWithImpl<CreateGroupWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateGroupWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'CreateGroupWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $CreateGroupWithStorageOutcome_FailureCopyWith<$Res> implements $CreateGroupWithStorageOutcomeCopyWith<$Res> {
  factory $CreateGroupWithStorageOutcome_FailureCopyWith(CreateGroupWithStorageOutcome_Failure value, $Res Function(CreateGroupWithStorageOutcome_Failure) _then) = _$CreateGroupWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$CreateGroupWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $CreateGroupWithStorageOutcome_FailureCopyWith<$Res> {
  _$CreateGroupWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final CreateGroupWithStorageOutcome_Failure _self;
  final $Res Function(CreateGroupWithStorageOutcome_Failure) _then;

/// Create a copy of CreateGroupWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(CreateGroupWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$DiscardPendingCommitWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DiscardPendingCommitWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'DiscardPendingCommitWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $DiscardPendingCommitWithStorageOutcomeCopyWith<$Res>  {
$DiscardPendingCommitWithStorageOutcomeCopyWith(DiscardPendingCommitWithStorageOutcome _, $Res Function(DiscardPendingCommitWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [DiscardPendingCommitWithStorageOutcome].
extension DiscardPendingCommitWithStorageOutcomePatterns on DiscardPendingCommitWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( DiscardPendingCommitWithStorageOutcome_Success value)?  success,TResult Function( DiscardPendingCommitWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case DiscardPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that);case DiscardPendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( DiscardPendingCommitWithStorageOutcome_Success value)  success,required TResult Function( DiscardPendingCommitWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case DiscardPendingCommitWithStorageOutcome_Success():
return success(_that);case DiscardPendingCommitWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( DiscardPendingCommitWithStorageOutcome_Success value)?  success,TResult? Function( DiscardPendingCommitWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case DiscardPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that);case DiscardPendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( DiscardPendingCommitWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case DiscardPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that.field0);case DiscardPendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( DiscardPendingCommitWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case DiscardPendingCommitWithStorageOutcome_Success():
return success(_that.field0);case DiscardPendingCommitWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( DiscardPendingCommitWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case DiscardPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that.field0);case DiscardPendingCommitWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class DiscardPendingCommitWithStorageOutcome_Success extends DiscardPendingCommitWithStorageOutcome {
  const DiscardPendingCommitWithStorageOutcome_Success(this.field0): super._();


@override final  DiscardPendingCommitWithStorageResult field0;

/// Create a copy of DiscardPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DiscardPendingCommitWithStorageOutcome_SuccessCopyWith<DiscardPendingCommitWithStorageOutcome_Success> get copyWith => _$DiscardPendingCommitWithStorageOutcome_SuccessCopyWithImpl<DiscardPendingCommitWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DiscardPendingCommitWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DiscardPendingCommitWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DiscardPendingCommitWithStorageOutcome_SuccessCopyWith<$Res> implements $DiscardPendingCommitWithStorageOutcomeCopyWith<$Res> {
  factory $DiscardPendingCommitWithStorageOutcome_SuccessCopyWith(DiscardPendingCommitWithStorageOutcome_Success value, $Res Function(DiscardPendingCommitWithStorageOutcome_Success) _then) = _$DiscardPendingCommitWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 DiscardPendingCommitWithStorageResult field0
});




}
/// @nodoc
class _$DiscardPendingCommitWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $DiscardPendingCommitWithStorageOutcome_SuccessCopyWith<$Res> {
  _$DiscardPendingCommitWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final DiscardPendingCommitWithStorageOutcome_Success _self;
  final $Res Function(DiscardPendingCommitWithStorageOutcome_Success) _then;

/// Create a copy of DiscardPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DiscardPendingCommitWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DiscardPendingCommitWithStorageResult,
  ));
}


}

/// @nodoc


class DiscardPendingCommitWithStorageOutcome_Failure extends DiscardPendingCommitWithStorageOutcome {
  const DiscardPendingCommitWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of DiscardPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DiscardPendingCommitWithStorageOutcome_FailureCopyWith<DiscardPendingCommitWithStorageOutcome_Failure> get copyWith => _$DiscardPendingCommitWithStorageOutcome_FailureCopyWithImpl<DiscardPendingCommitWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DiscardPendingCommitWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'DiscardPendingCommitWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $DiscardPendingCommitWithStorageOutcome_FailureCopyWith<$Res> implements $DiscardPendingCommitWithStorageOutcomeCopyWith<$Res> {
  factory $DiscardPendingCommitWithStorageOutcome_FailureCopyWith(DiscardPendingCommitWithStorageOutcome_Failure value, $Res Function(DiscardPendingCommitWithStorageOutcome_Failure) _then) = _$DiscardPendingCommitWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$DiscardPendingCommitWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $DiscardPendingCommitWithStorageOutcome_FailureCopyWith<$Res> {
  _$DiscardPendingCommitWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final DiscardPendingCommitWithStorageOutcome_Failure _self;
  final $Res Function(DiscardPendingCommitWithStorageOutcome_Failure) _then;

/// Create a copy of DiscardPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(DiscardPendingCommitWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$GetPendingCommitWithStorageOutcome {

 Object? get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GetPendingCommitWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'GetPendingCommitWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $GetPendingCommitWithStorageOutcomeCopyWith<$Res>  {
$GetPendingCommitWithStorageOutcomeCopyWith(GetPendingCommitWithStorageOutcome _, $Res Function(GetPendingCommitWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [GetPendingCommitWithStorageOutcome].
extension GetPendingCommitWithStorageOutcomePatterns on GetPendingCommitWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( GetPendingCommitWithStorageOutcome_Success value)?  success,TResult Function( GetPendingCommitWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case GetPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that);case GetPendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( GetPendingCommitWithStorageOutcome_Success value)  success,required TResult Function( GetPendingCommitWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case GetPendingCommitWithStorageOutcome_Success():
return success(_that);case GetPendingCommitWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( GetPendingCommitWithStorageOutcome_Success value)?  success,TResult? Function( GetPendingCommitWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case GetPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that);case GetPendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PendingCommitInfo? field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case GetPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that.field0);case GetPendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PendingCommitInfo? field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case GetPendingCommitWithStorageOutcome_Success():
return success(_that.field0);case GetPendingCommitWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PendingCommitInfo? field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case GetPendingCommitWithStorageOutcome_Success() when success != null:
return success(_that.field0);case GetPendingCommitWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class GetPendingCommitWithStorageOutcome_Success extends GetPendingCommitWithStorageOutcome {
  const GetPendingCommitWithStorageOutcome_Success([this.field0]): super._();


@override final  PendingCommitInfo? field0;

/// Create a copy of GetPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GetPendingCommitWithStorageOutcome_SuccessCopyWith<GetPendingCommitWithStorageOutcome_Success> get copyWith => _$GetPendingCommitWithStorageOutcome_SuccessCopyWithImpl<GetPendingCommitWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GetPendingCommitWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'GetPendingCommitWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $GetPendingCommitWithStorageOutcome_SuccessCopyWith<$Res> implements $GetPendingCommitWithStorageOutcomeCopyWith<$Res> {
  factory $GetPendingCommitWithStorageOutcome_SuccessCopyWith(GetPendingCommitWithStorageOutcome_Success value, $Res Function(GetPendingCommitWithStorageOutcome_Success) _then) = _$GetPendingCommitWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 PendingCommitInfo? field0
});




}
/// @nodoc
class _$GetPendingCommitWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $GetPendingCommitWithStorageOutcome_SuccessCopyWith<$Res> {
  _$GetPendingCommitWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final GetPendingCommitWithStorageOutcome_Success _self;
  final $Res Function(GetPendingCommitWithStorageOutcome_Success) _then;

/// Create a copy of GetPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = freezed,}) {
  return _then(GetPendingCommitWithStorageOutcome_Success(
freezed == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PendingCommitInfo?,
  ));
}


}

/// @nodoc


class GetPendingCommitWithStorageOutcome_Failure extends GetPendingCommitWithStorageOutcome {
  const GetPendingCommitWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of GetPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GetPendingCommitWithStorageOutcome_FailureCopyWith<GetPendingCommitWithStorageOutcome_Failure> get copyWith => _$GetPendingCommitWithStorageOutcome_FailureCopyWithImpl<GetPendingCommitWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GetPendingCommitWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'GetPendingCommitWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $GetPendingCommitWithStorageOutcome_FailureCopyWith<$Res> implements $GetPendingCommitWithStorageOutcomeCopyWith<$Res> {
  factory $GetPendingCommitWithStorageOutcome_FailureCopyWith(GetPendingCommitWithStorageOutcome_Failure value, $Res Function(GetPendingCommitWithStorageOutcome_Failure) _then) = _$GetPendingCommitWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$GetPendingCommitWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $GetPendingCommitWithStorageOutcome_FailureCopyWith<$Res> {
  _$GetPendingCommitWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final GetPendingCommitWithStorageOutcome_Failure _self;
  final $Res Function(GetPendingCommitWithStorageOutcome_Failure) _then;

/// Create a copy of GetPendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(GetPendingCommitWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$JoinGroupFromWelcomeWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is JoinGroupFromWelcomeWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'JoinGroupFromWelcomeWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $JoinGroupFromWelcomeWithStorageOutcomeCopyWith<$Res>  {
$JoinGroupFromWelcomeWithStorageOutcomeCopyWith(JoinGroupFromWelcomeWithStorageOutcome _, $Res Function(JoinGroupFromWelcomeWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [JoinGroupFromWelcomeWithStorageOutcome].
extension JoinGroupFromWelcomeWithStorageOutcomePatterns on JoinGroupFromWelcomeWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( JoinGroupFromWelcomeWithStorageOutcome_Success value)?  success,TResult Function( JoinGroupFromWelcomeWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case JoinGroupFromWelcomeWithStorageOutcome_Success() when success != null:
return success(_that);case JoinGroupFromWelcomeWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( JoinGroupFromWelcomeWithStorageOutcome_Success value)  success,required TResult Function( JoinGroupFromWelcomeWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case JoinGroupFromWelcomeWithStorageOutcome_Success():
return success(_that);case JoinGroupFromWelcomeWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( JoinGroupFromWelcomeWithStorageOutcome_Success value)?  success,TResult? Function( JoinGroupFromWelcomeWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case JoinGroupFromWelcomeWithStorageOutcome_Success() when success != null:
return success(_that);case JoinGroupFromWelcomeWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( JoinGroupWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case JoinGroupFromWelcomeWithStorageOutcome_Success() when success != null:
return success(_that.field0);case JoinGroupFromWelcomeWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( JoinGroupWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case JoinGroupFromWelcomeWithStorageOutcome_Success():
return success(_that.field0);case JoinGroupFromWelcomeWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( JoinGroupWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case JoinGroupFromWelcomeWithStorageOutcome_Success() when success != null:
return success(_that.field0);case JoinGroupFromWelcomeWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class JoinGroupFromWelcomeWithStorageOutcome_Success extends JoinGroupFromWelcomeWithStorageOutcome {
  const JoinGroupFromWelcomeWithStorageOutcome_Success(this.field0): super._();


@override final  JoinGroupWithStorageResult field0;

/// Create a copy of JoinGroupFromWelcomeWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWith<JoinGroupFromWelcomeWithStorageOutcome_Success> get copyWith => _$JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWithImpl<JoinGroupFromWelcomeWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is JoinGroupFromWelcomeWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'JoinGroupFromWelcomeWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWith<$Res> implements $JoinGroupFromWelcomeWithStorageOutcomeCopyWith<$Res> {
  factory $JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWith(JoinGroupFromWelcomeWithStorageOutcome_Success value, $Res Function(JoinGroupFromWelcomeWithStorageOutcome_Success) _then) = _$JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 JoinGroupWithStorageResult field0
});




}
/// @nodoc
class _$JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWith<$Res> {
  _$JoinGroupFromWelcomeWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final JoinGroupFromWelcomeWithStorageOutcome_Success _self;
  final $Res Function(JoinGroupFromWelcomeWithStorageOutcome_Success) _then;

/// Create a copy of JoinGroupFromWelcomeWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(JoinGroupFromWelcomeWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as JoinGroupWithStorageResult,
  ));
}


}

/// @nodoc


class JoinGroupFromWelcomeWithStorageOutcome_Failure extends JoinGroupFromWelcomeWithStorageOutcome {
  const JoinGroupFromWelcomeWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of JoinGroupFromWelcomeWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWith<JoinGroupFromWelcomeWithStorageOutcome_Failure> get copyWith => _$JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWithImpl<JoinGroupFromWelcomeWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is JoinGroupFromWelcomeWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'JoinGroupFromWelcomeWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWith<$Res> implements $JoinGroupFromWelcomeWithStorageOutcomeCopyWith<$Res> {
  factory $JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWith(JoinGroupFromWelcomeWithStorageOutcome_Failure value, $Res Function(JoinGroupFromWelcomeWithStorageOutcome_Failure) _then) = _$JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWith<$Res> {
  _$JoinGroupFromWelcomeWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final JoinGroupFromWelcomeWithStorageOutcome_Failure _self;
  final $Res Function(JoinGroupFromWelcomeWithStorageOutcome_Failure) _then;

/// Create a copy of JoinGroupFromWelcomeWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(JoinGroupFromWelcomeWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$MergePendingCommitWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MergePendingCommitWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'MergePendingCommitWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $MergePendingCommitWithStorageOutcomeCopyWith<$Res>  {
$MergePendingCommitWithStorageOutcomeCopyWith(MergePendingCommitWithStorageOutcome _, $Res Function(MergePendingCommitWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [MergePendingCommitWithStorageOutcome].
extension MergePendingCommitWithStorageOutcomePatterns on MergePendingCommitWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MergePendingCommitWithStorageOutcome_Success value)?  success,TResult Function( MergePendingCommitWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MergePendingCommitWithStorageOutcome_Success() when success != null:
return success(_that);case MergePendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MergePendingCommitWithStorageOutcome_Success value)  success,required TResult Function( MergePendingCommitWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case MergePendingCommitWithStorageOutcome_Success():
return success(_that);case MergePendingCommitWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MergePendingCommitWithStorageOutcome_Success value)?  success,TResult? Function( MergePendingCommitWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case MergePendingCommitWithStorageOutcome_Success() when success != null:
return success(_that);case MergePendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( MergePendingCommitWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MergePendingCommitWithStorageOutcome_Success() when success != null:
return success(_that.field0);case MergePendingCommitWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( MergePendingCommitWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case MergePendingCommitWithStorageOutcome_Success():
return success(_that.field0);case MergePendingCommitWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( MergePendingCommitWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case MergePendingCommitWithStorageOutcome_Success() when success != null:
return success(_that.field0);case MergePendingCommitWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class MergePendingCommitWithStorageOutcome_Success extends MergePendingCommitWithStorageOutcome {
  const MergePendingCommitWithStorageOutcome_Success(this.field0): super._();


@override final  MergePendingCommitWithStorageResult field0;

/// Create a copy of MergePendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergePendingCommitWithStorageOutcome_SuccessCopyWith<MergePendingCommitWithStorageOutcome_Success> get copyWith => _$MergePendingCommitWithStorageOutcome_SuccessCopyWithImpl<MergePendingCommitWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MergePendingCommitWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'MergePendingCommitWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $MergePendingCommitWithStorageOutcome_SuccessCopyWith<$Res> implements $MergePendingCommitWithStorageOutcomeCopyWith<$Res> {
  factory $MergePendingCommitWithStorageOutcome_SuccessCopyWith(MergePendingCommitWithStorageOutcome_Success value, $Res Function(MergePendingCommitWithStorageOutcome_Success) _then) = _$MergePendingCommitWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 MergePendingCommitWithStorageResult field0
});




}
/// @nodoc
class _$MergePendingCommitWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $MergePendingCommitWithStorageOutcome_SuccessCopyWith<$Res> {
  _$MergePendingCommitWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final MergePendingCommitWithStorageOutcome_Success _self;
  final $Res Function(MergePendingCommitWithStorageOutcome_Success) _then;

/// Create a copy of MergePendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(MergePendingCommitWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MergePendingCommitWithStorageResult,
  ));
}


}

/// @nodoc


class MergePendingCommitWithStorageOutcome_Failure extends MergePendingCommitWithStorageOutcome {
  const MergePendingCommitWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of MergePendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergePendingCommitWithStorageOutcome_FailureCopyWith<MergePendingCommitWithStorageOutcome_Failure> get copyWith => _$MergePendingCommitWithStorageOutcome_FailureCopyWithImpl<MergePendingCommitWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MergePendingCommitWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'MergePendingCommitWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $MergePendingCommitWithStorageOutcome_FailureCopyWith<$Res> implements $MergePendingCommitWithStorageOutcomeCopyWith<$Res> {
  factory $MergePendingCommitWithStorageOutcome_FailureCopyWith(MergePendingCommitWithStorageOutcome_Failure value, $Res Function(MergePendingCommitWithStorageOutcome_Failure) _then) = _$MergePendingCommitWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$MergePendingCommitWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $MergePendingCommitWithStorageOutcome_FailureCopyWith<$Res> {
  _$MergePendingCommitWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final MergePendingCommitWithStorageOutcome_Failure _self;
  final $Res Function(MergePendingCommitWithStorageOutcome_Failure) _then;

/// Create a copy of MergePendingCommitWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(MergePendingCommitWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$ProcessMessageWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProcessMessageWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'ProcessMessageWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $ProcessMessageWithStorageOutcomeCopyWith<$Res>  {
$ProcessMessageWithStorageOutcomeCopyWith(ProcessMessageWithStorageOutcome _, $Res Function(ProcessMessageWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [ProcessMessageWithStorageOutcome].
extension ProcessMessageWithStorageOutcomePatterns on ProcessMessageWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ProcessMessageWithStorageOutcome_Success value)?  success,TResult Function( ProcessMessageWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ProcessMessageWithStorageOutcome_Success() when success != null:
return success(_that);case ProcessMessageWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ProcessMessageWithStorageOutcome_Success value)  success,required TResult Function( ProcessMessageWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case ProcessMessageWithStorageOutcome_Success():
return success(_that);case ProcessMessageWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ProcessMessageWithStorageOutcome_Success value)?  success,TResult? Function( ProcessMessageWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case ProcessMessageWithStorageOutcome_Success() when success != null:
return success(_that);case ProcessMessageWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( ProcessMessageWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ProcessMessageWithStorageOutcome_Success() when success != null:
return success(_that.field0);case ProcessMessageWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( ProcessMessageWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case ProcessMessageWithStorageOutcome_Success():
return success(_that.field0);case ProcessMessageWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( ProcessMessageWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case ProcessMessageWithStorageOutcome_Success() when success != null:
return success(_that.field0);case ProcessMessageWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class ProcessMessageWithStorageOutcome_Success extends ProcessMessageWithStorageOutcome {
  const ProcessMessageWithStorageOutcome_Success(this.field0): super._();


@override final  ProcessMessageWithStorageResult field0;

/// Create a copy of ProcessMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProcessMessageWithStorageOutcome_SuccessCopyWith<ProcessMessageWithStorageOutcome_Success> get copyWith => _$ProcessMessageWithStorageOutcome_SuccessCopyWithImpl<ProcessMessageWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProcessMessageWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ProcessMessageWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ProcessMessageWithStorageOutcome_SuccessCopyWith<$Res> implements $ProcessMessageWithStorageOutcomeCopyWith<$Res> {
  factory $ProcessMessageWithStorageOutcome_SuccessCopyWith(ProcessMessageWithStorageOutcome_Success value, $Res Function(ProcessMessageWithStorageOutcome_Success) _then) = _$ProcessMessageWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 ProcessMessageWithStorageResult field0
});




}
/// @nodoc
class _$ProcessMessageWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $ProcessMessageWithStorageOutcome_SuccessCopyWith<$Res> {
  _$ProcessMessageWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final ProcessMessageWithStorageOutcome_Success _self;
  final $Res Function(ProcessMessageWithStorageOutcome_Success) _then;

/// Create a copy of ProcessMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ProcessMessageWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ProcessMessageWithStorageResult,
  ));
}


}

/// @nodoc


class ProcessMessageWithStorageOutcome_Failure extends ProcessMessageWithStorageOutcome {
  const ProcessMessageWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of ProcessMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProcessMessageWithStorageOutcome_FailureCopyWith<ProcessMessageWithStorageOutcome_Failure> get copyWith => _$ProcessMessageWithStorageOutcome_FailureCopyWithImpl<ProcessMessageWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProcessMessageWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ProcessMessageWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ProcessMessageWithStorageOutcome_FailureCopyWith<$Res> implements $ProcessMessageWithStorageOutcomeCopyWith<$Res> {
  factory $ProcessMessageWithStorageOutcome_FailureCopyWith(ProcessMessageWithStorageOutcome_Failure value, $Res Function(ProcessMessageWithStorageOutcome_Failure) _then) = _$ProcessMessageWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$ProcessMessageWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $ProcessMessageWithStorageOutcome_FailureCopyWith<$Res> {
  _$ProcessMessageWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final ProcessMessageWithStorageOutcome_Failure _self;
  final $Res Function(ProcessMessageWithStorageOutcome_Failure) _then;

/// Create a copy of ProcessMessageWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ProcessMessageWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$RemoveMembersWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is RemoveMembersWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'RemoveMembersWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $RemoveMembersWithStorageOutcomeCopyWith<$Res>  {
$RemoveMembersWithStorageOutcomeCopyWith(RemoveMembersWithStorageOutcome _, $Res Function(RemoveMembersWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [RemoveMembersWithStorageOutcome].
extension RemoveMembersWithStorageOutcomePatterns on RemoveMembersWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( RemoveMembersWithStorageOutcome_Success value)?  success,TResult Function( RemoveMembersWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case RemoveMembersWithStorageOutcome_Success() when success != null:
return success(_that);case RemoveMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( RemoveMembersWithStorageOutcome_Success value)  success,required TResult Function( RemoveMembersWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case RemoveMembersWithStorageOutcome_Success():
return success(_that);case RemoveMembersWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( RemoveMembersWithStorageOutcome_Success value)?  success,TResult? Function( RemoveMembersWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case RemoveMembersWithStorageOutcome_Success() when success != null:
return success(_that);case RemoveMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PendingCommitWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case RemoveMembersWithStorageOutcome_Success() when success != null:
return success(_that.field0);case RemoveMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PendingCommitWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case RemoveMembersWithStorageOutcome_Success():
return success(_that.field0);case RemoveMembersWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PendingCommitWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case RemoveMembersWithStorageOutcome_Success() when success != null:
return success(_that.field0);case RemoveMembersWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class RemoveMembersWithStorageOutcome_Success extends RemoveMembersWithStorageOutcome {
  const RemoveMembersWithStorageOutcome_Success(this.field0): super._();


@override final  PendingCommitWithStorageResult field0;

/// Create a copy of RemoveMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$RemoveMembersWithStorageOutcome_SuccessCopyWith<RemoveMembersWithStorageOutcome_Success> get copyWith => _$RemoveMembersWithStorageOutcome_SuccessCopyWithImpl<RemoveMembersWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is RemoveMembersWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'RemoveMembersWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $RemoveMembersWithStorageOutcome_SuccessCopyWith<$Res> implements $RemoveMembersWithStorageOutcomeCopyWith<$Res> {
  factory $RemoveMembersWithStorageOutcome_SuccessCopyWith(RemoveMembersWithStorageOutcome_Success value, $Res Function(RemoveMembersWithStorageOutcome_Success) _then) = _$RemoveMembersWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 PendingCommitWithStorageResult field0
});




}
/// @nodoc
class _$RemoveMembersWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $RemoveMembersWithStorageOutcome_SuccessCopyWith<$Res> {
  _$RemoveMembersWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final RemoveMembersWithStorageOutcome_Success _self;
  final $Res Function(RemoveMembersWithStorageOutcome_Success) _then;

/// Create a copy of RemoveMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(RemoveMembersWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PendingCommitWithStorageResult,
  ));
}


}

/// @nodoc


class RemoveMembersWithStorageOutcome_Failure extends RemoveMembersWithStorageOutcome {
  const RemoveMembersWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of RemoveMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$RemoveMembersWithStorageOutcome_FailureCopyWith<RemoveMembersWithStorageOutcome_Failure> get copyWith => _$RemoveMembersWithStorageOutcome_FailureCopyWithImpl<RemoveMembersWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is RemoveMembersWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'RemoveMembersWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $RemoveMembersWithStorageOutcome_FailureCopyWith<$Res> implements $RemoveMembersWithStorageOutcomeCopyWith<$Res> {
  factory $RemoveMembersWithStorageOutcome_FailureCopyWith(RemoveMembersWithStorageOutcome_Failure value, $Res Function(RemoveMembersWithStorageOutcome_Failure) _then) = _$RemoveMembersWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$RemoveMembersWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $RemoveMembersWithStorageOutcome_FailureCopyWith<$Res> {
  _$RemoveMembersWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final RemoveMembersWithStorageOutcome_Failure _self;
  final $Res Function(RemoveMembersWithStorageOutcome_Failure) _then;

/// Create a copy of RemoveMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(RemoveMembersWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$SelfUpdateWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SelfUpdateWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'SelfUpdateWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $SelfUpdateWithStorageOutcomeCopyWith<$Res>  {
$SelfUpdateWithStorageOutcomeCopyWith(SelfUpdateWithStorageOutcome _, $Res Function(SelfUpdateWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [SelfUpdateWithStorageOutcome].
extension SelfUpdateWithStorageOutcomePatterns on SelfUpdateWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SelfUpdateWithStorageOutcome_Success value)?  success,TResult Function( SelfUpdateWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SelfUpdateWithStorageOutcome_Success() when success != null:
return success(_that);case SelfUpdateWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SelfUpdateWithStorageOutcome_Success value)  success,required TResult Function( SelfUpdateWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case SelfUpdateWithStorageOutcome_Success():
return success(_that);case SelfUpdateWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SelfUpdateWithStorageOutcome_Success value)?  success,TResult? Function( SelfUpdateWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case SelfUpdateWithStorageOutcome_Success() when success != null:
return success(_that);case SelfUpdateWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PendingCommitWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SelfUpdateWithStorageOutcome_Success() when success != null:
return success(_that.field0);case SelfUpdateWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PendingCommitWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case SelfUpdateWithStorageOutcome_Success():
return success(_that.field0);case SelfUpdateWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PendingCommitWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case SelfUpdateWithStorageOutcome_Success() when success != null:
return success(_that.field0);case SelfUpdateWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class SelfUpdateWithStorageOutcome_Success extends SelfUpdateWithStorageOutcome {
  const SelfUpdateWithStorageOutcome_Success(this.field0): super._();


@override final  PendingCommitWithStorageResult field0;

/// Create a copy of SelfUpdateWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SelfUpdateWithStorageOutcome_SuccessCopyWith<SelfUpdateWithStorageOutcome_Success> get copyWith => _$SelfUpdateWithStorageOutcome_SuccessCopyWithImpl<SelfUpdateWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SelfUpdateWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'SelfUpdateWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $SelfUpdateWithStorageOutcome_SuccessCopyWith<$Res> implements $SelfUpdateWithStorageOutcomeCopyWith<$Res> {
  factory $SelfUpdateWithStorageOutcome_SuccessCopyWith(SelfUpdateWithStorageOutcome_Success value, $Res Function(SelfUpdateWithStorageOutcome_Success) _then) = _$SelfUpdateWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 PendingCommitWithStorageResult field0
});




}
/// @nodoc
class _$SelfUpdateWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $SelfUpdateWithStorageOutcome_SuccessCopyWith<$Res> {
  _$SelfUpdateWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final SelfUpdateWithStorageOutcome_Success _self;
  final $Res Function(SelfUpdateWithStorageOutcome_Success) _then;

/// Create a copy of SelfUpdateWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(SelfUpdateWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PendingCommitWithStorageResult,
  ));
}


}

/// @nodoc


class SelfUpdateWithStorageOutcome_Failure extends SelfUpdateWithStorageOutcome {
  const SelfUpdateWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of SelfUpdateWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SelfUpdateWithStorageOutcome_FailureCopyWith<SelfUpdateWithStorageOutcome_Failure> get copyWith => _$SelfUpdateWithStorageOutcome_FailureCopyWithImpl<SelfUpdateWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SelfUpdateWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'SelfUpdateWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $SelfUpdateWithStorageOutcome_FailureCopyWith<$Res> implements $SelfUpdateWithStorageOutcomeCopyWith<$Res> {
  factory $SelfUpdateWithStorageOutcome_FailureCopyWith(SelfUpdateWithStorageOutcome_Failure value, $Res Function(SelfUpdateWithStorageOutcome_Failure) _then) = _$SelfUpdateWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$SelfUpdateWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $SelfUpdateWithStorageOutcome_FailureCopyWith<$Res> {
  _$SelfUpdateWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final SelfUpdateWithStorageOutcome_Failure _self;
  final $Res Function(SelfUpdateWithStorageOutcome_Failure) _then;

/// Create a copy of SelfUpdateWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(SelfUpdateWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

/// @nodoc
mixin _$SwapMembersWithStorageOutcome {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapMembersWithStorageOutcome&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'SwapMembersWithStorageOutcome(field0: $field0)';
}


}

/// @nodoc
class $SwapMembersWithStorageOutcomeCopyWith<$Res>  {
$SwapMembersWithStorageOutcomeCopyWith(SwapMembersWithStorageOutcome _, $Res Function(SwapMembersWithStorageOutcome) __);
}


/// Adds pattern-matching-related methods to [SwapMembersWithStorageOutcome].
extension SwapMembersWithStorageOutcomePatterns on SwapMembersWithStorageOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SwapMembersWithStorageOutcome_Success value)?  success,TResult Function( SwapMembersWithStorageOutcome_Failure value)?  failure,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SwapMembersWithStorageOutcome_Success() when success != null:
return success(_that);case SwapMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SwapMembersWithStorageOutcome_Success value)  success,required TResult Function( SwapMembersWithStorageOutcome_Failure value)  failure,}){
final _that = this;
switch (_that) {
case SwapMembersWithStorageOutcome_Success():
return success(_that);case SwapMembersWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SwapMembersWithStorageOutcome_Success value)?  success,TResult? Function( SwapMembersWithStorageOutcome_Failure value)?  failure,}){
final _that = this;
switch (_that) {
case SwapMembersWithStorageOutcome_Success() when success != null:
return success(_that);case SwapMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PendingCommitWithStorageResult field0)?  success,TResult Function( MlsErrorCode field0)?  failure,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SwapMembersWithStorageOutcome_Success() when success != null:
return success(_that.field0);case SwapMembersWithStorageOutcome_Failure() when failure != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PendingCommitWithStorageResult field0)  success,required TResult Function( MlsErrorCode field0)  failure,}) {final _that = this;
switch (_that) {
case SwapMembersWithStorageOutcome_Success():
return success(_that.field0);case SwapMembersWithStorageOutcome_Failure():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PendingCommitWithStorageResult field0)?  success,TResult? Function( MlsErrorCode field0)?  failure,}) {final _that = this;
switch (_that) {
case SwapMembersWithStorageOutcome_Success() when success != null:
return success(_that.field0);case SwapMembersWithStorageOutcome_Failure() when failure != null:
return failure(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class SwapMembersWithStorageOutcome_Success extends SwapMembersWithStorageOutcome {
  const SwapMembersWithStorageOutcome_Success(this.field0): super._();


@override final  PendingCommitWithStorageResult field0;

/// Create a copy of SwapMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapMembersWithStorageOutcome_SuccessCopyWith<SwapMembersWithStorageOutcome_Success> get copyWith => _$SwapMembersWithStorageOutcome_SuccessCopyWithImpl<SwapMembersWithStorageOutcome_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapMembersWithStorageOutcome_Success&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'SwapMembersWithStorageOutcome.success(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $SwapMembersWithStorageOutcome_SuccessCopyWith<$Res> implements $SwapMembersWithStorageOutcomeCopyWith<$Res> {
  factory $SwapMembersWithStorageOutcome_SuccessCopyWith(SwapMembersWithStorageOutcome_Success value, $Res Function(SwapMembersWithStorageOutcome_Success) _then) = _$SwapMembersWithStorageOutcome_SuccessCopyWithImpl;
@useResult
$Res call({
 PendingCommitWithStorageResult field0
});




}
/// @nodoc
class _$SwapMembersWithStorageOutcome_SuccessCopyWithImpl<$Res>
    implements $SwapMembersWithStorageOutcome_SuccessCopyWith<$Res> {
  _$SwapMembersWithStorageOutcome_SuccessCopyWithImpl(this._self, this._then);

  final SwapMembersWithStorageOutcome_Success _self;
  final $Res Function(SwapMembersWithStorageOutcome_Success) _then;

/// Create a copy of SwapMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(SwapMembersWithStorageOutcome_Success(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PendingCommitWithStorageResult,
  ));
}


}

/// @nodoc


class SwapMembersWithStorageOutcome_Failure extends SwapMembersWithStorageOutcome {
  const SwapMembersWithStorageOutcome_Failure(this.field0): super._();


@override final  MlsErrorCode field0;

/// Create a copy of SwapMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapMembersWithStorageOutcome_FailureCopyWith<SwapMembersWithStorageOutcome_Failure> get copyWith => _$SwapMembersWithStorageOutcome_FailureCopyWithImpl<SwapMembersWithStorageOutcome_Failure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapMembersWithStorageOutcome_Failure&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'SwapMembersWithStorageOutcome.failure(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $SwapMembersWithStorageOutcome_FailureCopyWith<$Res> implements $SwapMembersWithStorageOutcomeCopyWith<$Res> {
  factory $SwapMembersWithStorageOutcome_FailureCopyWith(SwapMembersWithStorageOutcome_Failure value, $Res Function(SwapMembersWithStorageOutcome_Failure) _then) = _$SwapMembersWithStorageOutcome_FailureCopyWithImpl;
@useResult
$Res call({
 MlsErrorCode field0
});




}
/// @nodoc
class _$SwapMembersWithStorageOutcome_FailureCopyWithImpl<$Res>
    implements $SwapMembersWithStorageOutcome_FailureCopyWith<$Res> {
  _$SwapMembersWithStorageOutcome_FailureCopyWithImpl(this._self, this._then);

  final SwapMembersWithStorageOutcome_Failure _self;
  final $Res Function(SwapMembersWithStorageOutcome_Failure) _then;

/// Create a copy of SwapMembersWithStorageOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(SwapMembersWithStorageOutcome_Failure(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MlsErrorCode,
  ));
}


}

// dart format on
