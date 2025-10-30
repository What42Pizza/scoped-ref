use crate::*;
use std::marker::PhantomData;

#[cfg(feature = "runtime-none" )]
use std::sync::{Mutex, Condvar};
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
/// 
/// Also, this type only implements `Send` and/or `Sync` when the underlying reference implements `Send` and/or `Sync`
pub struct ScopedRefGuard<ConnectorType: TypeConnector> where [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {
	
	pub(crate) data_ptr: [u8; std::mem::size_of::<&ConnectorType::Super<'static>>()],
	
	// stores the counter and the notify together, which allows the `Arc<Notify>` when "no-pin" and "runtime-tokio" are used together
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
	pub(crate) counter_notify: (&'static AtomicU32, &'static Mutex<()>, &'static Condvar),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
	pub(crate) counter_notify: Arc<(Mutex<()>, Condvar)>,
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
	pub(crate) counter_notify: (&'static AtomicU32, &'static Notify),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
	pub(crate) counter_notify: Arc<Notify>,
	
	pub(crate) phantom: PhantomData<*mut ConnectorType>, // NOTE: the `*mut` is used to intentionally make `ScopedRefGuard` not Send/Sync
	
}

unsafe impl<ConnectorType: TypeConnector> Send for ScopedRefGuard<ConnectorType> where for<'a> <ConnectorType as TypeConnector>::Super<'a>: Send, [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {}
unsafe impl<ConnectorType: TypeConnector> Sync for ScopedRefGuard<ConnectorType> where for<'a> <ConnectorType as TypeConnector>::Super<'a>: Sync, [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {}

impl<ConnectorType: TypeConnector> ScopedRefGuard<ConnectorType> where [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {
	/// Returns the inner data. This is similar to `deref()` from the `Deref` trait, but is separate because it requires special lifetimes
	#[inline]
	pub fn inner<'a>(&'a self) -> &'a ConnectorType::Super<'a> {
		/*
		SAFETY (lifetime): the lifetime should be safe because
		1: the underlying data `T` can only be dropped after the `ScopedRef` referencing it is dropped
		2: the `ScopedRef` referencing `T` can only be dropped after all `ScopedRefGuards` created from it are dropped
		3: all `ScopedRefGuards` referencing `T` can only be dropped after all references to the guard are dropped, so
		4: `T` can only be dropped after all references to `T` given by this function are dropped
		*/
		unsafe {
			// SAFETY (size): the type for `data_ptr` ensures that it is the same size as `&ConnectorType::Super`
			&*(&self.data_ptr as *const _ as *const &'a ConnectorType::Super<'a>)
		}
	}
}

impl<ConnectorType: TypeConnector> Drop for ScopedRefGuard<ConnectorType> where [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {
	fn drop(&mut self) {
		#[cfg(not(feature = "no-pin"))]
		{
			let prev_count = self.counter_notify.0.fetch_sub(1, Ordering::AcqRel);
			if prev_count == 1 {
				#[cfg(feature = "runtime-none")]
				{
					// locking the mutex is necessary to prevent sending a notification after the main ScopedRef checks the active count but before it waits on the condvar
					let lock = self.counter_notify.1.lock().expect("failed to lock mutex while dropping data guard");
					self.counter_notify.2.notify_all();
					drop(lock);
				}
				#[cfg(feature = "runtime-tokio")]
				self.counter_notify.1.notify_waiters();
			}
		}
		#[cfg(feature = "no-pin")]
		{
			#[cfg(feature = "runtime-none")]
			if Arc::strong_count(&self.counter_notify) == 2 {
				// locking the mutex is necessary to prevent sending a notification after the main ScopedRef checks the active count but before it waits on the condvar
				let lock = self.counter_notify.0.lock().expect("failed to lock mutex while dropping data guard");
				self.counter_notify.1.notify_all();
				drop(lock);
			}
			#[cfg(feature = "runtime-tokio")]
			if Arc::strong_count(&self.counter_notify) == 2 {
				self.counter_notify.notify_waiters();
			}
		}
	}
}

impl<ConnectorType: TypeConnector> std::fmt::Debug for ScopedRefGuard<ConnectorType> where for<'a> ConnectorType::Super<'a>: std::fmt::Debug, [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.inner().fmt(f)
	}
}

impl<ConnectorType: TypeConnector> std::fmt::Display for ScopedRefGuard<ConnectorType> where for<'a> ConnectorType::Super<'a>: std::fmt::Display, [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.inner().fmt(f)
	}
}

impl<ConnectorType: TypeConnector> Clone for ScopedRefGuard<ConnectorType> where [(); std::mem::size_of::<&ConnectorType::Super<'static>>()]: Sized {
	#[inline]
	fn clone(&self) -> Self {
		#[cfg(not(feature = "no-pin"))]
		self.counter_notify.0.fetch_add(1, Ordering::AcqRel);
		Self {
			data_ptr: self.data_ptr,
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
			counter_notify: self.counter_notify,
			#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
			counter_notify: self.counter_notify.clone(),
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
			counter_notify: self.counter_notify,
			#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
			counter_notify: self.counter_notify.clone(),
			phantom: PhantomData,
		}
	}
}
