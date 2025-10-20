# Scoped Ref

A fast, lightweight, and safe way to use non-`'static` data where `'static` is expected. This gives functionality similar to `std::thread::scoped()`, but this can be used anywhere, including async code.

### Example usage: (not using async here, but the "runtime-tokio" feature is used by default)

```rust
let my_huge_data: Vec<u8> = get_huge_data();

{
	// Step 1: create the scope
	make_type_connector!(SliceU8 = <'a> [u8]);
	let scoped_data = ScopedRef::<SliceU8>::new(&*my_huge_data);
	let scoped_data = std::pin::pin!(scoped_data);
	
	// Step 2: use the scope
	let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
	std::thread::spawn(move || {
		do_processing(data_ref.deref());
	});
	let data_ref: ScopedRefGuard<SliceU8> = scoped_data.new_ref();
	std::thread::spawn(move || {
		do_more_processing(data_ref.deref());
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

### This crate should be safe because:

- The data referenced by `ScopedRef` cannot be dropped until the `ScopedRef` is dropped (enforced by Rust)
- A `ScopedRef` cannot be dropped until all guards created from it are dropped (enforced with reference counting)
- A `ScopedRefGuard` cannot be dropped until all references to it are dropped (enforced by Rust and this crate's api)
- You cannot keep any references to a `ScopedRefGuard` by tricking the lifetime, because the signature (simplified) for retrieving the inner data is `fn deref<'a>(&'a self) -> &'a InnerData<'a>`
- A `ScopedRefGuard<T>` only implements Send and/or Sync if `T` implements Send any/or Sync

Although, I'm very experienced with unsafe code (which this crate uses a fair bit), so if you are experienced with preventing unsafe bugs, I'd highly appreciate extra safety reviews!