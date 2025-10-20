//! Scoped Static
//! 
//! Similar functionality to the lifetimes of `std::thread::scope()`, but available everywhere. Allows non-'static lifetimes to be promoted to 'static in a safe and extremely fast manner (theoretically faster than `Arc<T>`, but not yet tested)
//! 
//! Advantages over Arc:
//! - No heap allocation
//! - Less reference counters (1 counter instead of 2)
//! - Less counting in general (only when a `ScopedRefGuard` is created/dropped)
//! 
//! ### Example usage:
//! 
//! ```
//! use scoped_static::*;
//! fn spawn_threads_with_data(my_huge_data: Vec<u8>) {
//!     {
//!         // Create a type to represent our data
//!         make_type_connector!(SliceU8 = <'a> [u8]);
//!         // Create a scope for the data. This can be used to safely access the inner data as if it is `'static`
//!         let scoped_data = ScopedRef::<SliceU8>::new(&*my_huge_data);
//!         // Pin the scope (necessary for crate functionality)
//!         let scoped_data = std::pin::pin!(scoped_data);
//!         
//!         // Create a `ScopedRefGuard` that can be passed to anything that takes `'static` data
//!         let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
//!         std::thread::spawn(move || {
//!             do_processing(data_ref.deref());
//!         });
//!         
//!         // The first `data_ref` has been moved, create a separate one for this separate task
//!         let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
//!         std::thread::spawn(move || {
//!             do_more_processing(data_ref.deref());
//!         });
//!         
//!         // Optionally, you can block on your own terms
//!         // This gives you the option of setting a timeout, where you can set how many times it sleeps before returning `Err(())`
//!         scoped_data.await_guards(Some(std::time::Duration::from_hours(1)));
//!         let did_finished = !scoped_data.has_active_guards(); // if you give `None` to `await_guards` then `has_active_guards` should always return false (unless you call `new_ref()` in between)
//!         
//!     }
//!     // because the `pin!()`, you can only drop `scoped_data` by going out of scope
//!     // and doing so ensures that all references to `my_huge_data` are dropped before continuing
//!     
//!     drop(my_huge_data);
//!     
//! }
//! fn do_processing(my_huge_data: &[u8]) {}
//! fn do_more_processing(my_huge_data: &[u8]) {}
//! ```
//! 
//! ### External runtimes:
//! 
//! This can work with either a standard runtime or a Tokio runtime, and if more options are needed then they can surely be added. To use this crate with Tokio, you should enable this crate's "runtime-tokio" feature



#![warn(missing_docs)]



use std::{marker::PhantomData, pin::Pin, sync::{atomic::{AtomicU32, Ordering}}, time::Duration};
#[cfg(feature = "runtime-std")]
use crossbeam::sync::{Parker, Unparker};
#[cfg(feature = "runtime-tokio")]
use tokio::{runtime::Handle, sync::Notify};

#[cfg(feature = "runtime-tokio")]
pub use tokio;



/// Allows you to create `ScopedRefGuards`, which can send non-`'static` data to anything that requires `'static` data.
/// 
/// This works because the static-friendly guards prevent their parent `ScopeRef` from being dropped, meaning their data can always be accessed as if it is static. The resulting functionality is similar to lifetimes superpowers of `std::thread::scope()`, but available everywhere
pub struct ScopedRef<'a, ConnectorType: TypeConnector> {
	
	pub(crate) data_ptr: ConnectorType::RawPointerStorage, // SAFETY: the raw data inside this var must be of the type &'a ConnectorType::Super<'a>
	pub(crate) counter: AtomicU32,
	
	#[cfg(feature = "runtime-std")]
	pub(crate) notify: Parker,
	#[cfg(feature = "runtime-tokio")]
	pub(crate) notify: Notify,
	
	pub(crate) phantom: PhantomData<&'a ConnectorType>,
	
}

impl<'a, ConnectorType: TypeConnector> ScopedRef<'a, ConnectorType> {
	
	/// Creates a new `ScopedRef`. NOTE: you must `pin!()` the returned value for it to be usable!
	pub fn new(data: impl Into<&'a ConnectorType::Super<'a>>) -> Self where &'a ConnectorType::Super<'a>: Copy {
		#[cfg(all(debug_assertions, feature = "runtime-tokio"))]
		{
			Handle::current(); // check whether this is being called within a valid tokio runtime (only checks in debug mode, exists bc the drop fn already needs the handle and seeing the panic in `new()` is probably better than in the drop)
		}
		let mut output = Self {
			data_ptr: ConnectorType::RAW_POINTER_DEFAULT,
			counter: AtomicU32::new(0),
			#[cfg(feature = "runtime-std")]
			notify: Parker::new(),
			#[cfg(feature = "runtime-tokio")]
			notify: Notify::new(),
			phantom: PhantomData,
		};
		let data_ptr: &'a ConnectorType::Super<'a> = data.into();
		unsafe {
			debug_assert!(std::mem::size_of::<ConnectorType::RawPointerStorage>() >= std::mem::size_of::<&'a ConnectorType::Super<'a>>(), "Undefined behaviour prevented: not enough storage for the given reference. In the call to the `make_connector_type!()` macro, please edit (or add) the size needed");
			*(&mut output.data_ptr as *mut _ as *mut &'a ConnectorType::Super<'a>) = data_ptr;
		}
		output
	}
	
	/// Returns a new guard that can be used to access `&T` as if it is `&'static T`
	/// 
	/// As you can see from the function signature, the `ScopedRef` has to be `pin!()`ed before this function can be called. This is due to the atomic counter in `ScopedRef`, which must always stay in the same location for `ScopedRefGuard` to properly access it
	pub fn new_ref(self: &Pin<&mut Self>) -> ScopedRefGuard<ConnectorType> {
		self.counter.fetch_add(1, Ordering::AcqRel);
		ScopedRefGuard {
			data_ptr: self.data_ptr,
			counter: unsafe {&*(&self.counter as *const _)}, // SAFETY: same safety as data_ptr, see `ScopedRefGuard::deref()` for reasoning
			#[cfg(feature = "runtime-std")]
			notify: self.notify.unparker().clone(),
			#[cfg(feature = "runtime-tokio")]
			notify: unsafe {&*(&self.notify as *const _)}, // SAFETY: same safety as data_ptr, see `ScopedRefGuard::deref()` for reasoning
			phantom: PhantomData,
		}
	}
	
	/// Blocks until all guards have been dropped (is async on async runtimes)
	#[cfg(feature = "runtime-std")]
	pub fn await_guards(&self, timeout: Option<Duration>) {
		if !self.has_active_guards() { return; }
		if let Some(timeout) = timeout {
			self.notify.park_timeout(timeout);
		} else {
			self.notify.park();
		}
	}
	/// Blocks until all guards have been dropped (is async on async runtimes)
	#[cfg(feature = "runtime-tokio")]
	pub async fn await_guards(&self, timeout: Option<Duration>) {
		if !self.has_active_guards() { return; }
		if let Some(timeout) = timeout {
			let notify_future = self.notify.notified();
			let _possible_notify_future = tokio::time::timeout(timeout, notify_future).await;
		} else {
			self.notify.notified().await;
		}
	}
	
	/// Returns whether there are still living `ScopedRefGuard`s that would cause dropping this `ScopedRef` to block
	pub fn has_active_guards(&self) -> bool {
		self.counter.load(Ordering::Acquire) > 0
	}
	
}

// When ScopedRef is dropped, it must wait until all ScopedRefGuards have been dropped before continuing execution
impl<'a, ConnectorType: TypeConnector> Drop for ScopedRef<'a, ConnectorType> {
	fn drop(&mut self) {
		#[cfg(feature = "runtime-std")]
		{
			self.await_guards(None);
		}
		#[cfg(feature = "runtime-tokio")]
		{
			tokio::task::block_in_place(move || {
				Handle::current().block_on((async || {
					self.await_guards(None).await;
				})())
			});
		}
	}
}



/// Similar to something like `MutexGuard`, but for keeping track of the number of references to `T`.
/// 
/// A `ScopedRefGuard` can only be dropped once all references to it are dropped, and a `ScopedRef` can only be dropped once all `ScopedRefGuard`s have been dropped, and the underlying data `T` can only be dropped once the `ScopedRef` referencing it has been dropped
pub struct ScopedRefGuard<ConnectorType: TypeConnector> {
	
	pub(crate) data_ptr: ConnectorType::RawPointerStorage, // SAFETY: the raw data inside this var must be of the type &'a ConnectorType::Super<'a>
	pub(crate) counter: &'static AtomicU32,
	
	#[cfg(feature = "runtime-std")]
	pub(crate) notify: Unparker,
	#[cfg(feature = "runtime-tokio")]
	pub(crate) notify: &'static Notify,
	
	pub(crate) phantom: PhantomData<*mut ConnectorType>, // NOTE: the `*mut` is used to intentionally make `ScopedRefGuard` not Send/Sync
	
}

unsafe impl<T> Send for ScopedRefGuard<T> where T: TypeConnector, for<'a> <T as TypeConnector>::Super<'a>: Send {}
unsafe impl<T> Sync for ScopedRefGuard<T> where T: TypeConnector, for<'a> <T as TypeConnector>::Super<'a>: Sync {}

impl<ConnectorType: TypeConnector> ScopedRefGuard<ConnectorType> {
	/// Returns the inner data. This does not use the `Deref` trait because this requires special lifetimes
	pub fn deref<'a>(&'a self) -> &'a ConnectorType::Super<'a> {
		/*
		SAFETY (lifetime): the lifetime should be safe because
		1: the underlying data `T` can only be dropped after the `ScopedRef` referencing it is dropped
		2: the `ScopedRef` referencing `T` can only be dropped after all `ScopedRefGuards` created from it are dropped
		3: all `ScopedRefGuards` referencing `T` can only be dropped after all references to the guard are dropped, so
		4: `T` can only be dropped after all references to `T` given by this function are dropped
		*/
		unsafe {
			// SAFETY (reading): a `ScopedRefGuard` can only be made with `ScopedRef::new()`, which already implements a check to make sure this has enough space to store `&'a ConnectorType::Super<'a>`
			std::ptr::read(&self.data_ptr as *const _ as *const &'a ConnectorType::Super<'a>)
		}
	}
}

impl<ConnectorType: TypeConnector> Drop for ScopedRefGuard<ConnectorType> {
	fn drop(&mut self) {
		let prev_count = self.counter.fetch_sub(1, Ordering::AcqRel);
		if prev_count == 1 {
			#[cfg(feature = "runtime-std")]
			self.notify.unpark();
			#[cfg(feature = "runtime-tokio")]
			self.notify.notify_waiters();
		}
	}
}

impl<ConnectorType: TypeConnector> std::fmt::Debug for ScopedRefGuard<ConnectorType> where for<'a> ConnectorType::Super<'a>: std::fmt::Debug {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.deref().fmt(f)
	}
}

impl<ConnectorType: TypeConnector> std::fmt::Display for ScopedRefGuard<ConnectorType> where for<'a> ConnectorType::Super<'a>: std::fmt::Display {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.deref().fmt(f)
	}
}

impl<'a, ConnectorType: TypeConnector> Clone for ScopedRefGuard<ConnectorType> {
	fn clone(&self) -> Self {
		self.counter.fetch_add(1, Ordering::AcqRel);
		Self {
			data_ptr: self.data_ptr,
			counter: self.counter,
			#[cfg(feature = "runtime-std")]
			notify: self.notify.clone(),
			#[cfg(feature = "runtime-tokio")]
			notify: self.notify,
			phantom: PhantomData,
		}
	}
}



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



#[cfg(test)]
mod tests {
    use crate::*;
	use std::{thread, pin::pin};
	
	#[cfg(feature = "runtime-std")]
	#[test]
	fn basic_test() {
		let data = String::from("Test Data");
		{
			make_type_connector!(RefString *1 = <'a> String);
			let scoped_data = ScopedRef::<RefString>::new(&data);
			let scoped_data = pin!(scoped_data);
			
			let data_ref = scoped_data.new_ref();
			thread::spawn(move || {
				println!("Sleeping for 1 second...");
				thread::sleep(Duration::from_secs(1));
				println!("Data: {data_ref}");
			});
		}
		
		println!("All threads finished!");
	}
	#[cfg(feature = "runtime-tokio")]
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn basic_test() {
		let data = String::from("Test Data");
		{
			make_type_connector!(RefString *1 = <'a> String);
			let scoped_data = ScopedRef::<RefString>::new(&data);
			let scoped_data = pin!(scoped_data);
			
			let data_ref = scoped_data.new_ref();
			thread::spawn(move || {
				println!("Sleeping for 1 second...");
				thread::sleep(Duration::from_secs(1));
				println!("Data: {data_ref}");
			});
		}
		
		println!("All threads finished!");
	}
	
	#[cfg(feature = "runtime-std")]
	#[test]
	fn advanced_type_test() {
		struct AdvancedType<'a> {
			inner: &'a u8,
		}
		let inner = 128;
		let data = AdvancedType {
			inner: &inner,
		};
		{
			make_type_connector!(RefAdvancedType *1 = <'a> AdvancedType<'a>);
			let scoped_data = ScopedRef::<RefAdvancedType>::new(&data);
			let scoped_data = pin!(scoped_data);
			
			let data_ref = scoped_data.new_ref();
			thread::spawn(move || {
				println!("Sleeping for 1 second...");
				thread::sleep(Duration::from_secs(1));
				println!("Data: {}", data_ref.deref().inner);
			});
		}
		
		println!("All threads finished!");
	}
	#[cfg(feature = "runtime-tokio")]
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn advanced_type_test() {
		struct AdvancedType<'a> {
			inner: &'a u8,
		}
		let inner = 128;
		let data = AdvancedType {
			inner: &inner,
		};
		{
			make_type_connector!(RefAdvancedType *1 = <'a> AdvancedType<'a>);
			let scoped_data = ScopedRef::<RefAdvancedType>::new(&data);
			let scoped_data = pin!(scoped_data);
			
			let data_ref = scoped_data.new_ref();
			thread::spawn(move || {
				println!("Sleeping for 1 second...");
				thread::sleep(Duration::from_secs(1));
				println!("Data: {}", data_ref.deref().inner);
			});
		}
		
		println!("All threads finished!");
	}
	
	#[cfg(feature = "runtime-std")]
	#[test]
	fn test_macro() {
		
		make_type_connector!(MyType *5 = <'a> Vec<&'a u8>);
		
		let inner_data = 0u8;
		let _: <MyType as TypeConnector>::Super<'_> = vec!(&inner_data);
		let _: <MyType as TypeConnector>::RawPointerStorage = [0; 5];
		let _: <MyType as TypeConnector>::RawPointerStorage = <MyType as TypeConnector>::RAW_POINTER_DEFAULT;
		
	}
	#[cfg(feature = "runtime-tokio")]
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn test_macro() {
		
		make_type_connector!(MyType *5 = <'a> Vec<&'a u8>);
		
		let inner_data = 0u8;
		let _: <MyType as TypeConnector>::Super<'_> = vec!(&inner_data);
		let _: <MyType as TypeConnector>::RawPointerStorage = [0; 5];
		let _: <MyType as TypeConnector>::RawPointerStorage = <MyType as TypeConnector>::RAW_POINTER_DEFAULT;
		
	}
	
	#[cfg(feature = "runtime-std")]
	#[test]
	fn test_std_traits() {
		
		make_type_connector!(SliceU8 = <'a> [u8]);
		let data = vec!(1, 2, 3);
		let scoped_data = ScopedRef::<SliceU8>::new(&*data);
		let scoped_data = std::pin::pin!(scoped_data);
		let data_ref = scoped_data.new_ref();
		assert_eq!(format!("{data_ref:?}"), String::from("[1, 2, 3]"));
		
		make_type_connector!(U8 = <'a> u8);
		let data = 123;
		let scoped_data = ScopedRef::<U8>::new(&data);
		let scoped_data = std::pin::pin!(scoped_data);
		let data_ref = scoped_data.new_ref();
		assert_eq!(format!("{data_ref}"), String::from("123"));
		
		let data_ref_2 = data_ref.clone();
		assert_eq!(scoped_data.counter.load(Ordering::Acquire), 2);
		drop(data_ref);
		drop(data_ref_2);
		
	}
	#[cfg(feature = "runtime-tokio")]
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn test_std_traits() {
		
		make_type_connector!(SliceU8 = <'a> [u8]);
		let data = vec!(1, 2, 3);
		let scoped_data = ScopedRef::<SliceU8>::new(&*data);
		let scoped_data = std::pin::pin!(scoped_data);
		let data_ref = scoped_data.new_ref();
		assert_eq!(format!("{data_ref:?}"), String::from("[1, 2, 3]"));
		
		make_type_connector!(U8 = <'a> u8);
		let data = 123;
		let scoped_data = ScopedRef::<U8>::new(&data);
		let scoped_data = std::pin::pin!(scoped_data);
		let data_ref = scoped_data.new_ref();
		assert_eq!(format!("{data_ref}"), String::from("123"));
		
		let data_ref_2 = data_ref.clone();
		assert_eq!(scoped_data.counter.load(Ordering::Acquire), 2);
		drop(data_ref);
		drop(data_ref_2);
		
	}
	
}
