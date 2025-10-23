//! # Scoped Ref
//! 
//! Similar functionality to the lifetimes of `std::thread::scope()`, but available everywhere. Allows non-`'static` lifetimes to be promoted to `'static` in a safe and extremely fast manner (theoretically faster than `Arc<T>` (when not using the "no-pin" feature), but not yet tested)
//! 
//! ### Example usage:
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
//!     let scoped_data = ScopedRef::<SliceU8>::new(&*my_huge_data);
//!     // Pin the scope (needed unless the "no-pin" feature is enabled)
//!     let scoped_data = std::pin::pin!(scoped_data);
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



#![warn(missing_docs)]



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



#[cfg(not(any(feature = "runtime-none", feature = "runtime-tokio")))]
compile_error!("At least one of these features must be enabled for the scoped-ref crate: \"runtime-none\", \"runtime-tokio\"");
#[cfg(not(any(feature = "drop-does-block", feature = "unsafe-drop-does-panic", feature = "unsafe-drop-does-nothing")))]
compile_error!("At least one of these features must be enabled for the scoped-ref crate: \"drop-does-block\", \"unsafe-drop-does-panic\", \"unsafe-drop-does-nothing\"");
