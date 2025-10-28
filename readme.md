# Scoped Ref

A fast, lightweight, and safe way to use non-`'static` data where `'static` is expected. This gives functionality similar to `std::thread::scoped()`, but this can be used anywhere (including async code).

### Example usage:

```rust
let my_huge_data: Vec<u8> = get_huge_data();

{
	// Step 1: create the scope
	make_type_connector!(SliceU8 = <'a> [u8]);
	make_scoped_ref!(scoped_ref = (&*my_huge_data) as Sliceu8);
	
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

### Crate feature flags:

- `"runtime-none"`: Specifies using no special runtime
- `"runtime-tokio"` *: Specifies using the tokio runtime
- `"no-pin"`: Allows more flexibility (by not pinning the `ScopedRef`), but adds heap allocation
- `"drop-does-block"` *: Causes the drop function of `ScopedRef` to block until all guards have been dropped
- `"unsafe-drop-does-panic"`: Causes the drop function of `ScopedRef` to panic if there are still any guards active (this is considered unsafe because when it does panic, the unwind will always create dangling pointers)
- `"unsafe-drop-does-nothing"`: Causes the drop function of `ScopedRef` to do nothing, even if there are still guards active.
- `"unwind-does-abort"` *: Causes `ScopedRef` to abort the program if dropped during a panic unwind. This is to ensure no danging pointers are created
- `"unsafe-ignore-unwind"`: This is the opposite of the "unwind-does-abort" feature. If it is enabled, `ScopedRef`'s drop function will not check for unwinds and will proceed as dictated by the 'drop-does-' features

* = enabled by default

### Potential problems:

- There might be some situations where `ScopedRef`'s drop function could block indefinitely, but that is likely better than potentially creating dangling pointers. This can be changed by enabling a different 'drop-does-' feature.
- By design, this crate aborts the program if a `ScopedRef` in dropped because of an unwind. This is to ensure no dangling pointers are created on unwind. This can be changed by enabling the "unsafe-ignore-unwind" feature (and disabling the "unwind-does-abort" feature).
