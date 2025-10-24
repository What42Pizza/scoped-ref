use crate::*;
use std::{time::Duration, marker::PhantomData};

#[cfg(feature = "runtime-none")]
use std::{sync::{Mutex, Condvar}, time::Instant};
#[cfg(feature = "runtime-tokio")]
use tokio::{runtime::Handle, sync::Notify};

#[cfg(not(feature = "no-pin"))]
use std::{sync::atomic::{Ordering, AtomicU32}, pin::Pin};
#[cfg(feature = "no-pin")]
use std::sync::Arc;



/// Allows you to create runtime-checked scope where a non-`'static` reference can be used as if it is `'static`.
/// 
/// This works because the static-friendly guards prevent their parent `ScopeRef` from being dropped, meaning their data can always be accessed as if it is static. The resulting functionality is similar to lifetimes superpowers of `std::thread::scope()`, but available everywhere
pub struct ScopedRef<'a, ConnectorType: TypeConnector> {
	
	pub(crate) data_ptr: ConnectorType::RawPointerStorage, // SAFETY: the raw data inside this var must be of the type &'a ConnectorType::Super<'a>
	
	// stores the counter and the notify together, which allows the `Arc<Notify>` when "no-pin" and "runtime-tokio" are used together
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-none" ))]
	pub(crate) counter_notify: (AtomicU32, Mutex<()>, Condvar),
	#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
	pub(crate) counter_notify: Arc<(Mutex<()>, Condvar)>,
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
			counter_notify: (AtomicU32::new(0), Mutex::new(()), Condvar::new()),
			#[cfg(all(    feature = "no-pin" , feature = "runtime-none" ))]
			counter_notify: Arc::new((Mutex::new(()), Condvar::new())),
			#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
			counter_notify: (AtomicU32::new(0), Notify::new()),
			#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
			counter_notify: Arc::new(Notify::new()),
			
			phantom: PhantomData,
		};
		let data_ptr: &'a ConnectorType::Super<'a> = data.into();
		unsafe {
			assert!(std::mem::size_of::<ConnectorType::RawPointerStorage>() >= std::mem::size_of::<&'a ConnectorType::Super<'a>>(), "Undefined behaviour prevented: not enough storage for the given reference. In the call to the `make_connector_type!()` macro, please edit (or add) the size needed");
			*(&mut output.data_ptr as *mut _ as *mut &'a ConnectorType::Super<'a>) = data_ptr;
		}
		output
	}
	
	/// Returns a new guard that can be used to access `&T` as if it is `&'static T`
	/// 
	/// As you can see from the function signature, the `ScopedRef` has to be `pin!()`ed before this function can be called. This is due to the atomic counter in `ScopedRef`, which must always stay in the same location for `ScopedRefGuard` to properly access it
	#[cfg(not(feature = "no-pin"))]
	pub fn new_ref(self: &Pin<&mut Self>) -> ScopedRefGuard<ConnectorType> {
		self.counter_notify.0.fetch_add(1, Ordering::AcqRel);
		ScopedRefGuard {
			data_ptr: self.data_ptr,
			#[cfg(feature = "runtime-none" )]
			counter_notify: (unsafe {&*(&self.counter_notify.0 as *const _)}, unsafe {&*(&self.counter_notify.1 as *const _)}, unsafe {&*(&self.counter_notify.2 as *const _)}),
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
		ScopedRefGuard {
			data_ptr: self.data_ptr,
			counter_notify: self.counter_notify.clone(),
			phantom: PhantomData,
		}
	}
	
	/// Blocks until all guards have been dropped (is async on async runtimes)
	#[cfg(feature = "runtime-none")]
	pub fn await_guards(&self, timeout: Option<Duration>) {
		#[cfg(not(feature = "no-pin"))]
		let (mutex, condvar) = (&self.counter_notify.1, &self.counter_notify.2);
		#[cfg(feature = "no-pin")]
		let (mutex, condvar) = (&self.counter_notify.0, &self.counter_notify.1);
		if let Some(timeout) = timeout {
			
			let mut guard = mutex.lock().expect("failed to start waiting for data guards to drop");
			if !self.has_active_guards() { return; } // doing this here ensures that a notification can't be sent after this check but before the `condvar.wait()`
			let end = Instant::now() + timeout;
			(guard, _) = condvar.wait_timeout(guard, timeout).expect("failed to wait for data guards to drop");
			if !self.has_active_guards() { return; }
			loop {
				let now = Instant::now();
				if now > end { return; }
				(guard, _) = condvar.wait_timeout(guard, end - now).expect("failed to wait for data guards to drop");
				if !self.has_active_guards() { return; }
			}
			
		} else {
			
			let mut guard = mutex.lock().expect("failed to start waiting for data guards to drop");
			loop {
				if !self.has_active_guards() { return; }
				guard = condvar.wait(guard).expect("failed to wait for data guards to drop");
			}
			
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
		{ Arc::strong_count(&self.counter_notify) > 1 }
		#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
		{ self.counter_notify.0.load(Ordering::Acquire) > 0}
		#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
		{ Arc::strong_count(&self.counter_notify) > 1 }
	}
	
}

// When `ScopedRef` is dropped, it must wait until all `ScopedRefGuards` have been dropped before continuing execution (unless a different feature is enabled)
impl<'a, ConnectorType: TypeConnector> Drop for ScopedRef<'a, ConnectorType> {
	fn drop(&mut self) {
		#[cfg(feature = "unwind-does-abort")]
		if std::thread::panicking() {
			eprintln!("Program must be aborted due to a `ScopedRef` being dropped on unwind.");
			std::process::abort();
		}
		#[cfg(feature = "drop-does-block")]
		{
			#[cfg(feature = "runtime-none")]
			{
				self.await_guards(None);
			}
			#[cfg(feature = "runtime-tokio")]
			{
				tokio::task::block_in_place(move || {
					Handle::current().block_on(async {
						self.await_guards(None).await;
					})
				});
			}
		}
		#[cfg(feature = "unsafe-drop-does-panic")]
		{
			if !self.has_active_guards() { panic!("Attempting to drop a `ScopedRef` while it still has active guards"); }
		}
		#[cfg(feature = "unsafe-drop-does-nothing")]
		{}
	}
}
