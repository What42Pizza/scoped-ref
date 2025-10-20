# Scoped Ref

Similar functionality to `std::thread::scoped()`, but available everywhere.

This was primarily made for use with the tokio runtime (which requires the "runtime-tokio" feature), but it can be used with no runtime (which requires the "runtime-std" feature), or others if others are requested.

Also, you might want to check out these crates:
- [https://crates.io/crates/scoped_static](Scoped_Static) (most similar, but potentially slower)
- [https://crates.io/crates/async-scoped](ASync-Scoped) (most similar to `std:thread::scope()`, but more limited)

Similar to [https://crates.io/crates/scoped_static](this), but improved in many ways
