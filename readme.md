# Scoped Ref

A fast, lightweight, and safe way to use non-`'static` data where `'static` is expected. This gives functionality similar to `std::thread::scoped()`, but this can be used anywhere (including async code).

### Example usage: (not using async here, but the "runtime-tokio" feature is used by default)

```rust
let my_huge_data: Vec<u8> = get_huge_data();

{
	// Step 1: create the scope
	make_type_connector!(SliceU8 = <'a> [u8]);
	let scoped_data = ScopedRef::<SliceU8>::new(&*my_huge_data);
	let scoped_data = std::pin::pin!(scoped_data); // needed unless the "no-pin" feature is enabled
	
	// Step 2: use the scope
	let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
	std::thread::spawn(move || {
		let my_huge_data = data_ref.inner();
		do_processing(my_huge_data);
	});
	
	let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
	std::thread::spawn(move || {
		let my_huge_data = data_ref.inner();
		do_processing(my_huge_data);
	});
	
	// Step 3: drop the scope, which blocks until all guards are dropped
	
} // because the `pin!()`, you can only drop `scoped_data` by going out of scope

// this is safe to do because `ScopedRef` ensures no references to it remain after dropping
drop(my_huge_data);
```

### Comparisons to other similar crates:

| other crate | why use other crate? | why use this crate? |
|-------------|----------------------|---------------------|
| [Futures-Scopes](https://crates.io/crates/futures-scopes) | Directly tracks futures (instead of guards) | Likely faster |
| [Scoped_Static](https://crates.io/crates/scoped_static) | Likely safer and easier | Likely faster |
| [Async-Scoped](https://crates.io/crates/async-scoped) | Same as `std::thread::scope` but async | Likely faster |
| [Extend_Mut](https://crates.io/crates/extend_mut) | Less overhead | Much more flexible and async api is safe |

### Why is this crate likely faster than most others?

There are only three sources of overhead, which are:
- Creating a new guard, which is a single atomic operation
- Dropping a guard, which is a single atomic operation plus a cross-thread notify on last guard drop
- Dropping a ScopedRef, which sleeps until notified (or immediately continues if there's already no remaining guards)

### Crate features:

- `"runtime-none"`: Specifies using no special runtime
- `"runtime-tokio"`: Specifies using the tokio runtime (enabled by default)
- `"no-pin"`: Allows more flexibility (no pinning required), but adds heap allocation
- `"drop-does-block"`: Causes the drop function of `ScopedRef` to block until all guards have been dropped (enabled by default)
- `"unsafe-drop-does-panic"`: Causes the drop function of `ScopedRef` to panic if there are still have guards active (this is considered unsafe because when it does panic, the unwind will essentially always deallocate data that is still being used)
- `"unsafe-drop-does-nothing"`: Causes the drop function of `ScopedRef` to do nothing, even if there are still guards active. 

### Potential problems:

The biggest potential problem is that the drop function of `ScopedRef` may block indefinitely, but that is likely preferable to have a drop function that might lead to other resources being dropped while still being referenced. If 

Although, I'm very experienced with unsafe code (which this crate uses a fair bit), so if you are experienced with preventing unsafe bugs, I'd highly appreciate extra safety reviews!