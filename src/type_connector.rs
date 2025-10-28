//! Rust has a really annoying restriction that almost stops this crate from being useful in a lot of scenarios, which is the fact that if a type can a type parameter with non-`'static` references in it, the entire type becomes non-`'static`, even if the type's internals and functionality should be `'static`. Even this struct: `struct MyStruct<T> (PhantomData<T>);` will not be `'static` unless `T` is `'static`. This means you cannot use a type like `ScopedRefGuard<&'a [u8]>` (which would hold a reference to `&&'a [u8]`) because the mere presence of the non-`'static` lifetime makes the entire guard non-`'static`.
//! 
//! To get around this limitation, `ScopedRef` and `ScopedRefGuard` use a marker type to just represent the actual type being referenced. This is how that works:
//! 
//! - 1: `ScopedRef` and `ScopedRefGuard` both take any type that implements the [TypeConnector] trait
//! - 2: The `TypeConnector` trait has an associated type that defines the actual type being referenced
//! - 3: When `ScopedRef` or `ScopedRefGuard` need to access the actual type being referenced, they just use `TypeConnector::Super`
//!   - For example, just look at [ScopedRefGuard::inner()], which has the signature `fn inner(&self) -> &ConnectorType::Super` (note: `TypeConnector` is the name of the trait, and `ConnectorType` is the name of the generic that implements `TypeConnector`)



/// A type meant solely for enforcing type safety. To create this type, the [make_scoped_ref] macro is recommended
pub trait TypeConnector: 'static {
	/// This specifies the type that this `TypeConnector` represents, minus the leading `&` (so if you want to represent something like `&&u8`, this type should be `&u8`)
	type Super<'a>: ?Sized;
}



/// This is a utility for creating structs that implement [TypeConnector]
/// 
/// ### What is `TypeConnector` and why does it exist?
/// 
/// The `ScopedRef` and `ScopedRefGuard` structs need to share a generic type input so that type safety can be enforced, but using something like `&&u8` would cause `ScopedRefGuard` to be non-`'static`. That defeats the entire point of this crate, so instead, `ScopedRef` and `ScopedRefGuard` share a type that just represents the actual shared type.
/// 
/// ### Syntax:
/// 
/// ```ignore
/// make_type_connector!(ConnectorTypeName = <'a> TypeToBeReferenced);
/// ```
/// 
/// There are three inputs, which are:
/// - ConnectorTypeName: This will be the name of the helper struct that implements [TypeConnector]
/// - lifetime (the `<'a>`): This is for defining lifetimes within `TypeToBeReferenced`
/// - TypeToBeReferenced: This is simple the type that you want to feed to `ScopedRef::new()`, but minus the leading `&`
/// 
/// ### Examples:
/// 
/// ```ignore
/// 
/// // basic usage
/// make_type_connector!(RefCustomType = <'a> CustomType);
/// let scoped_data = ScopedRef::<RefCustomType>::new(&CustomType {..});
/// 
/// // referencing a slice
/// make_type_connector!(RefCustomTypeSlice = <'a> [CustomType]);
/// let scoped_data = ScopedRef::<RefCustomTypeSlice>::new(&[CustomType {..}]);
/// 
/// // referencing a str
/// make_type_connector!(RefStr = <'a> str);
/// let scoped_data = ScopedRef::<RefStr>::new("example");
/// 
/// // referencing a reference
/// make_type_connector!(RefRefCustomType = <'a> &'a CustomType);
/// let scoped_data = ScopedRef::<RefRefCustomType>::new(&&CustomType {..});
/// 
/// // referencing a type with inner references
/// make_type_connector!(RefAdvancedTypeRefU8 = <'a> AdvancedType<&'a u8>);
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
		
		struct $name;
		
		impl TypeConnector for $name {
			type Super<$lifetime> = $type;
		}
		
	};
}
