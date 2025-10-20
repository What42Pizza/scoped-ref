# Scoped Ref

Similar functionality to `std::thread::scoped()`, but async-compatible, available everywhere, and keeps maximum performance. (usable with or without async/await)

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

drop(my_huge_data);
```

Comparisons to other similar crates:

| other crate | why use other crate? | why use this crate? |
|-------------|----------------------|---------------------|
| [Futures-Scopes](https://crates.io/crates/futures-scopes) | Directly tracks futures (instead of guards) | Likely faster |
| [Scoped_Static](https://crates.io/crates/scoped_static) | Likely safer and easier | Likely faster |
| [Async-Scoped](https://crates.io/crates/async-scoped) | Same as `std::thread::scope` but async | Likely faster |

This was primarily made for use with the tokio runtime (which requires the "runtime-tokio" feature), but it can be used with no runtime (which requires the "runtime-std" feature), or others if others are requested.