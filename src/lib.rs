//! # Scoped Ref
//! 
//! Similar functionality to the lifetimes of `std::thread::scope()`, but available everywhere. Allows non-`'static` lifetimes to be promoted to `'static` in a safe and extremely fast manner (theoretically faster than `Arc<T>` (when not using the "no-pin" feature), but not yet tested)
//! 
//! ### Example usage:
//! 
//! This example is non-async, but this crate does support async/await by default
//! 
//! ```
//! use ::scoped_ref::*;
//! fn do_processing(my_huge_data: &[u8]) {}
//! fn do_more_processing(my_huge_data: &[u8]) {}
//! fn get_huge_data() -> Vec<u8> {vec!()}
//! 
//! let my_huge_data: Vec<u8> = get_huge_data();
//! // because the `pin!()`, you can only drop `scoped_data` by going out of scope
//! #[cfg(feature = "runtime-none")] {
//!     
//!     // Create a type to represent our data
//!     make_type_connector!(SliceU8 = <'a> [u8]);
//!     // Create a scope for the data. This can be used to safely access the inner data as if it is `'static`
//!     make_scoped_ref!(scoped_data = (&*my_huge_data) as SliceU8);
//!     
//!     // Create a `ScopedRefGuard` that can be passed to anything that takes `'static` data
//!     let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
//!     std::thread::spawn(move || {
//!         let my_huge_data = data_ref.inner();
//!         do_processing(my_huge_data);
//!     });
//!     
//!     // Create another `ScopedRefGuard` since the first was moved to the first thread
//!     let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
//!     std::thread::spawn(move || {
//!         let my_huge_data = data_ref.inner();
//!         do_more_processing(my_huge_data);
//!     });
//!     
//!     // If you want, you can choose when it blocks waiting for created guards to drop
//!     // This also gives you the option to set a timeout
//!     scoped_data.await_guards(Some(std::time::Duration::from_hours(1)));
//!     let did_finished = !scoped_data.has_active_guards(); // if you give `None` to `await_guards` then `has_active_guards` should always return false (unless you call `new_ref()` in between)
//!     
//! }
//! 
//! // dropping a `ScopedRef` blocks until all guards created from it are dropped, meaning the given data can only ever be dropped or mutated after all references/guards for it are dropped
//! drop(my_huge_data);
//! ```
//! 
//! ## External runtimes:
//! 
//! This can currently work with:
//! 
//! - **No runtime** if the `"runtime-none"` feature is enabled
//! - **The tokio runtime** if the `"runtime-tokio"` feature is enabled
//! 
//! If support for more runtimes is needed, just open an issue and adding it should be fairly simple
//! 
//! ## Feature flags:
//! 
//! - `"runtime-none"`: Specifies using no special runtime
//! - `"runtime-tokio"` *: Specifies using the tokio runtime
//! - `"no-pin"`: Allows more flexibility (by not pinning the `ScopedRef`), but adds heap allocation
//! - `"drop-does-block"` *: Causes the drop function of `ScopedRef` to block until all guards have been dropped
//! - `"drop-does-abort"`: Causes the drop function of `ScopedRef` to abort if there are still any guards active
//! - `"unsafe-drop-does-panic"`: Causes the drop function of `ScopedRef` to panic if there are still any guards active (this is considered unsafe because when it does panic, the unwind will always create dangling pointers)
//! - `"unsafe-drop-does-nothing"`: Causes the drop function of `ScopedRef` to do nothing, even if there are still guards active.
//! - `"unwind-does-abort"` *: Causes `ScopedRef` to abort the program if dropped during a panic unwind. This is to ensure no danging pointers are created
//! - `"unsafe-ignore-unwind"`: This is the opposite of the "unwind-does-abort" feature. If it is enabled, `ScopedRef`'s drop function will not check for unwinds and will proceed as dictated by the 'drop-does-' features
//! 
//! '*' = enabled by default



#![warn(missing_docs)]
#![forbid(clippy::unwrap_used)]

#![allow(incomplete_features)]
#![feature(generic_const_exprs)]



/// Everything about the `ScopedRef` type
pub mod scoped_ref;
pub use scoped_ref::*;
/// Everything about the `ScopedRefGuard` type
pub mod scoped_ref_guard;
pub use scoped_ref_guard::*;
/// Everything about the `TypeConnector` trait and macro
pub mod type_connector;
pub use type_connector::*;
mod tests;

#[cfg(feature = "runtime-tokio")]
pub use tokio;



// ensure features are used correctly:

#[allow(unused_mut)]
const _: () = {
	
	let mut runtime_count = 0;
	#[cfg(feature = "runtime-none")]
	{ runtime_count += 1; }
	#[cfg(feature = "runtime-tokio")]
	{ runtime_count += 1; }
	match runtime_count {
		0 => panic!("At least one of these features must be enabled in the `scoped-ref` crate: \"runtime-none\" or \"runtime-tokio\""),
		1 => {}
		_ => panic!("Only one of these features may be enabled in the `scoped-ref` crate: \"runtime-none\" or \"runtime-tokio\" (be sure to check default features)"),
	}
	
	let mut drop_count = 0;
	#[cfg(feature = "drop-does-block")]
	{ drop_count += 1; }
	#[cfg(feature = "drop-does-abort")]
	{ drop_count += 1; }
	#[cfg(feature = "unsafe-drop-does-panic")]
	{ drop_count += 1; }
	#[cfg(feature = "unsafe-drop-does-nothing")]
	{ drop_count += 1; }
	match drop_count {
		0 => panic!("At least one of these features must be enabled in the `scoped-ref` crate: \"drop-does-block\", \"drop-does-abort\", \"unsafe-drop-does-panic\", or \"unsafe-drop-does-nothing\""),
		1 => {}
		_ => panic!("Only one of these features may be enabled in the `scoped-ref` crate: \"drop-does-block\", \"drop-does-abort\", \"unsafe-drop-does-panic\", or \"unsafe-drop-does-nothing\" (be sure to check default features)"),
	}
	
	let mut unwind_count = 0;
	#[cfg(feature = "unwind-does-abort")]
	{ unwind_count += 1; }
	#[cfg(feature = "unsafe-ignore-unwind")]
	{ unwind_count += 1; }
	match unwind_count {
		0 => panic!("At least one of these features must be enabled in the `scoped-ref` crate: \"unwind-does-abort\" or \"unsafe-ignore-unwind\""),
		1 => {}
		_ => panic!("Only one of these features may be enabled in the `scoped-ref` crate: \"unwind-does-abort\" or \"unsafe-ignore-unwind\" (be sure to check default features)"),
	}
	
	#[cfg(all(feature = "tokio", not(feature = "runtime-tokio")))]
	panic!("The \"tokio\" feature of the `scoped-ref` crate must not be used directly, use \"runtime-tokio\" instead");
	
};
