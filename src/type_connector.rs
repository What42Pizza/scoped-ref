/// A type meant solely for enforcing type safety. To create this type, please go to [make_type_connector](make_type_connector!())
pub trait TypeConnector: 'static {
	/// This specifies the type that this `TypeConnector` represents
	type Super<'a>: ?Sized;
	/// This specifies how the reference passed to `ScopedRef::new()` is stored internally. It must be big enough for the entire reference!
	type RawPointerStorage: Copy;
	/// This is just the default value that is held before the pointer is copied in
	const RAW_POINTER_DEFAULT: Self::RawPointerStorage;
}

/// This is a utility for creating structs that implement [TypeConnector](TypeConnector)
/// 
/// ### What is `TypeConnector` and why does it exist?
/// 
/// The `ScopedRef` and `ScopedRefGuard` structs need to share a type to enforce type safety, but putting something like `&&u8` would force `ScopedRefGuard` to be non-`'static`. That defeats the entire point of this crate, so instead, `ScopedRef` and `ScopedRefGuard` share a type that represents the actual shared type.
/// 
/// ### Syntax:
/// 
/// ```ignore
/// make_type_connector!(ConnectorTypeName *1 = <'a> TypeToBeReferenced);
/// make_type_connector!(ConnectorTypeName = <'a> TypeToBeReferenced); // same as above but with `*4`
/// ```
/// 
/// There are three four inputs (with one being optional), which are:
/// - ConnectorTypeName: This will be the name of the helper struct that implements [TypeConnector](TypeConnector)
/// - reference length (the `*1`): This will be the length (in usize-s) of the reference to `TypeToBeReferenced`. For something like `&MyType`, the value should be *1, and for fat pointers (something like `&[MyType]`), the value should be *2
///   - WARNING: Setting this value too low is UB, and the size of references in rust is subject to change. The value given here likely will never need to be anything larger than 2, but to be safe, the default value is 4
/// - lifetime (the `<'a>`): This is for defining lifetimes within `TypeToBeReferenced`
/// - TypeToBeReferenced: This is simple the type that you want to feed to `ScopedRef::new()`, but minus the front `&`
/// 
/// ### Examples:
/// 
/// ```ignore
/// // Note: none of these need to specify *1 or *2, it's just for the example and you should probably leave it blank
/// 
/// // basic usage
/// make_type_connector!(RefCustomType *1 = <'a> CustomType);
/// let scoped_data = ScopedRef::<RefCustomType>::new(&CustomType {..});
/// 
/// // referencing a fat pointer (slice)
/// make_type_connector!(RefCustomTypeSlice *2 = <'a> [CustomType]);
/// let scoped_data = ScopedRef::<RefCustomTypeSlice>::new(&[CustomType {..}]);
/// 
/// // referencing a fat pointer (str)
/// make_type_connector!(RefStr *2 = <'a> str);
/// let scoped_data = ScopedRef::<RefStr>::new("example");
/// 
/// // referencing a reference
/// make_type_connector!(RefRefCustomType *1 = <'a> &'a CustomType);
/// let scoped_data = ScopedRef::<RefRefCustomType>::new(&&CustomType {..});
/// 
/// // referencing a type with inner references
/// make_type_connector!(RefAdvancedTypeRefU8 *1 = <'a> AdvancedType<&'a u8>);
/// let scoped_data = ScopedRef::<RefAdvancedTypeRefU8>::new(&AdvancedType {&0});
/// 
/// // and of course, you can (and probably should) leave the 'reference length' blank
/// make_type_connector!(RefSomeType = <'a> SomeType);
/// let scoped_data = ScopedRef::<RefSomeType>::new(&SomeType);
/// 
/// ```
#[macro_export]
macro_rules! make_type_connector {
	($name:ident = <$lifetime:tt> $type:ty) => {
		make_type_connector!($name *4 = <$lifetime> $type);
	};
	($name:ident *$storage_count:tt = <$lifetime:tt> $type:ty) => {
		
		struct $name;
		
		impl TypeConnector for $name {
			type Super<$lifetime> = $type;
			type RawPointerStorage = [usize; $storage_count];
			const RAW_POINTER_DEFAULT: Self::RawPointerStorage = [0; $storage_count];
		}
		
	};
}
