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
//! use ::scoped_ref::*;
//! fn spawn_threads_with_data(my_huge_data: Vec<u8>) {
//!     {
//!         // Create a type to represent our data
//!         make_type_connector!(SliceU8 = <'a> [u8]);
//!         // Create a scope for the data. This can be used to safely access the inner data as if it is `'static`
//!         let scoped_data = ScopedRef::<SliceU8>::new(&*my_huge_data);
//!         // Pin the scope (needed unless the "no-pin" feature is enabled)
//!         let scoped_data = std::pin::pin!(scoped_data);
//!         
//!         // Create a `ScopedRefGuard` that can be passed to anything that takes `'static` data
//!         let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
//!         std::thread::spawn(move || {
//!             let my_huge_data = data_ref.inner();
//!             do_processing(my_huge_data);
//!         });
//!         
//!         // Create another `ScopedRefGuard` since the first was moved to the first thread
//!         let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
//!         std::thread::spawn(move || {
//!             let my_huge_data = data_ref.inner();
//!             do_more_processing(my_huge_data);
//!         });
//!         
//!         // If you want, you can choose when it blocks waiting for created guards to drop
//!         // This also gives you the option to set a timeout
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
