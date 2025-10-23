use crate::*;
use std::{time::Duration, marker::PhantomData};

#[cfg(feature = "runtime-none" )]
use crossbeam::sync::Parker;
#[cfg(feature = "runtime-tokio")]
use tokio::{runtime::Handle, sync::Notify};

#[cfg(not(feature = "no-pin"))]
use std::{sync::atomic::{Ordering, AtomicU32}, pin::Pin};
#[cfg(feature = "no-pin")]
use std::sync::Arc;



/// Allows you to create `Scope
/// 
/// dRefGuards`, which can send non-`'static` data to anything that requires `'static` data.
/// 
/// This works because the static-friendly guards prevent their parent `ScopeRef` from being dropped, meaning their data can always be accessed as if it is static. The resulting functionality is similar to lifetimes superpowers of `std::thread::scope()`, but available everywhere
pub struct ScopedRef<'a, ConnectorType: TypeConnector> {
	
	pub(crate) data_ptr: ConnectorType::RawPointerStorage, // SAFETY: the raw data inside this var must be of the type &'a ConnectorType::Super<'a>
	
	// stores the counter and the notify together, which allows the `Arc<Notify>` when "no-pin" and "runtime-tokio" are used together
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
	pub(crate) counter_notify: (AtomicU32, Parker),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
	pub(crate) counter_notify: (Arc<()>, Parker),
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
	pub(crate) counter_notify: (AtomicU32, Notify),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
	pub(crate) counter_notify: Arc<Notify>,
	
	pub(crate) phantom: PhantomData<&'a ConnectorType>,
	
}

impl<'a, ConnectorType: TypeConnector> ScopedRef<'a, ConnectorType> {
	
	/// Creates a new `ScopedRef`. NOTE: you must `pin!()` the returned value for it to be usable! (unless the "no-pin" feature is enabled)
	pub fn new(data: impl Into<&'a ConnectorType::Super<'a>>) -> Self where &'a ConnectorType::Super<'a>: Copy {
		#[cfg(all(debug_assertions, feature = "runtime-tokio"))]
		{
			Handle::current(); // check whether this is being called within a valid tokio runtime (only checks in debug mode, exists bc the drop fn already needs the handle and seeing the panic in `new()` is probably better than in the drop)
		}
		let mut output = Self {
			data_ptr: ConnectorType::RAW_POINTER_DEFAULT,
			
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
			counter_notify: (AtomicU32::new(0), Parker::new()),
			#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
			counter_notify: (Arc::new(()), Parker::new()),
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
			counter_notify: (AtomicU32::new(0), Notify::new()),
			#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
			counter_notify: Arc::new(Notify::new()),
			
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
	#[cfg(not(feature = "no-pin"))]
	pub fn new_ref(self: &Pin<&mut Self>) -> ScopedRefGuard<ConnectorType> {
		#[cfg(not(feature = "no-pin"))]
		self.counter_notify.0.fetch_add(1, Ordering::AcqRel);
		ScopedRefGuard {
			data_ptr: self.data_ptr,
			#[cfg(feature = "runtime-none" )]
			counter_notify: (unsafe {&*(&self.counter_notify.0 as *const _)}, self.counter_notify.1.unparker().clone()),
			#[cfg(feature = "runtime-tokio")]
			counter_notify: (unsafe {&*(&self.counter_notify.0 as *const _)}, unsafe {&*(&self.counter_notify.1 as *const _)}),
			phantom: PhantomData,
		}
	}
	/// Returns a new guard that can be used to access `&T` as if it is `&'static T`
	/// 
	/// As you can see from the function signature, the `ScopedRef` has to be `pin!()`ed before this function can be called. This is due to the atomic counter in `ScopedRef`, which must always stay in the same location for `ScopedRefGuard` to properly access it
	#[cfg(feature = "no-pin")]
	pub fn new_ref(&self) -> ScopedRefGuard<ConnectorType> {
		#[cfg(not(feature = "no-pin"))]
		self.counter_notify.0.fetch_add(1, Ordering::AcqRel);
		ScopedRefGuard {
			data_ptr: self.data_ptr,
			#[cfg(feature = "runtime-none" )]
			counter_notify: (self.counter_notify.0.clone(), self.counter_notify.1.unparker().clone()),
			#[cfg(feature = "runtime-tokio")]
			counter_notify: self.counter_notify.clone(),
			phantom: PhantomData,
		}
	}
	
	/// Blocks until all guards have been dropped (is async on async runtimes)
	#[cfg(feature = "runtime-none")]
	pub fn await_guards(&self, timeout: Option<Duration>) {
		if !self.has_active_guards() { return; }
		if let Some(timeout) = timeout {
			self.counter_notify.1.park_timeout(timeout);
		} else {
			self.counter_notify.1.park();
		}
	}
	/// Blocks until all guards have been dropped (is async on async runtimes)
	#[cfg(feature = "runtime-tokio")]
	pub async fn await_guards(&self, timeout: Option<Duration>) {
		if !self.has_active_guards() { return; }
		#[cfg(not(feature = "no-pin"))]
		let notify = &self.counter_notify.1;
		#[cfg(feature = "no-pin")]
		let notify = &*self.counter_notify;
		if let Some(timeout) = timeout {
			let notify_future = notify.notified();
			let _possible_notify_future = tokio::time::timeout(timeout, notify_future).await;
		} else {
			notify.notified().await;
		}
	}
	
	/// Returns whether there are still living `ScopedRefGuard`s that would cause dropping this `ScopedRef` to block
	pub fn has_active_guards(&self) -> bool {
		#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
		{ self.counter_notify.0.load(Ordering::Acquire) > 0}
		#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
		{ Arc::strong_count(&self.counter_notify.0) > 1 }
		#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
		{ self.counter_notify.0.load(Ordering::Acquire) > 0}
		#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
		{ Arc::strong_count(&self.counter_notify) > 1 }
	}
	
}

// When ScopedRef is dropped, it must wait until all ScopedRefGuards have been dropped before continuing execution
impl<'a, ConnectorType: TypeConnector> Drop for ScopedRef<'a, ConnectorType> {
	fn drop(&mut self) {
		#[cfg(feature = "drop-does-block")]
		{
			#[cfg(feature = "runtime-none")]
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
		#[cfg(feature = "unsafe-drop-does-panic")]
		{
			if !self.has_active_guards() { panic!("Attempting to drop a `ScopedRef` while it still has active guards"); }
		}
		#[cfg(not(any(feature = "drop-does-block", feature = "unsafe-drop-does-panic", feature = "unsafe-drop-does-nothing")))]
		compile_error!("At least one of these features must be enabled for the scoped-ref crate: \"drop-does-block\", \"unsafe-drop-does-panic\", \"unsafe-drop-does-nothing\"");
	}
}
