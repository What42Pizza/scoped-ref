use crate::*;
use std::marker::PhantomData;

#[cfg(feature = "runtime-none" )]
use crossbeam::sync::Unparker;
#[cfg(feature = "runtime-tokio")]
use tokio::sync::Notify;

#[cfg(not(feature = "no-pin"))]
use std::sync::atomic::AtomicU32;
#[cfg(not(feature = "no-pin"))]
use std::sync::atomic::Ordering;
#[cfg(feature = "no-pin")]
use std::sync::Arc;



/// Similar to something like `MutexGuard`, but for keeping track of the number of references to `T`.
/// 
/// A `ScopedRefGuard` can only be dropped once all references to it are dropped, and a `ScopedRef` can only be dropped once all `ScopedRefGuard`s have been dropped, and the underlying data `T` can only be dropped once the `ScopedRef` referencing it has been dropped
pub struct ScopedRefGuard<ConnectorType: TypeConnector> {
	
	pub(crate) data_ptr: ConnectorType::RawPointerStorage, // SAFETY: the raw data inside this var must be of the type &'a ConnectorType::Super<'a>
	
	// stores the counter and the notify together, which allows the `Arc<Notify>` when "no-pin" and "runtime-tokio" are used together
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
	pub(crate) counter_notify: (&'static AtomicU32, Unparker),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
	pub(crate) counter_notify: (Arc<()>, Unparker),
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
	pub(crate) counter_notify: (&'static AtomicU32, &'static Notify),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
	pub(crate) counter_notify: Arc<Notify>,
	
	pub(crate) phantom: PhantomData<*mut ConnectorType>, // NOTE: the `*mut` is used to intentionally make `ScopedRefGuard` not Send/Sync
	
}

unsafe impl<T> Send for ScopedRefGuard<T> where T: TypeConnector, for<'a> <T as TypeConnector>::Super<'a>: Send {}
unsafe impl<T> Sync for ScopedRefGuard<T> where T: TypeConnector, for<'a> <T as TypeConnector>::Super<'a>: Sync {}

impl<ConnectorType: TypeConnector> ScopedRefGuard<ConnectorType> {
	/// Returns the inner data. This is similar to `deref()` from the `Deref` trait, but is separate because it requires special lifetimes
	pub fn inner<'a>(&'a self) -> &'a ConnectorType::Super<'a> {
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
		#[cfg(not(feature = "no-pin"))]
		{
			let prev_count = self.counter_notify.0.fetch_sub(1, Ordering::AcqRel);
			if prev_count == 1 {
				#[cfg(feature = "runtime-none")]
				self.counter_notify.1.unpark();
				#[cfg(feature = "runtime-tokio")]
				self.counter_notify.1.notify_waiters();
			}
		}
		#[cfg(feature = "no-pin")]
		{
			#[cfg(feature = "runtime-none")]
			if Arc::strong_count(&self.counter_notify.0) == 2 {
				self.counter_notify.1.unpark();
			}
			#[cfg(feature = "runtime-tokio")]
			if Arc::strong_count(&self.counter_notify) == 2 {
				self.counter_notify.notify_waiters();
			}
		}
	}
}

impl<ConnectorType: TypeConnector> std::fmt::Debug for ScopedRefGuard<ConnectorType> where for<'a> ConnectorType::Super<'a>: std::fmt::Debug {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.inner().fmt(f)
	}
}

impl<ConnectorType: TypeConnector> std::fmt::Display for ScopedRefGuard<ConnectorType> where for<'a> ConnectorType::Super<'a>: std::fmt::Display {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.inner().fmt(f)
	}
}

impl<ConnectorType: TypeConnector> Clone for ScopedRefGuard<ConnectorType> {
	fn clone(&self) -> Self {
		#[cfg(not(feature = "no-pin"))]
		self.counter_notify.0.fetch_add(1, Ordering::AcqRel);
		Self {
			data_ptr: self.data_ptr,
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
			counter_notify: (self.counter_notify.0, self.counter_notify.1.clone()),
			#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
			counter_notify: (self.counter_notify.0.clone(), self.counter_notify.1.clone()),
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
			counter_notify: self.counter_notify,
			#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
			counter_notify: self.counter_notify.clone(),
			phantom: PhantomData,
		}
	}
}
