#[allow(unused_imports)] // idk why rust says this is unused
use crate::*;



/*
To test all situations, run:
cargo test
cargo test --no-default-features --features runtime-none,drop-does-block
cargo test --features no-pin
cargo test --no-default-features --features runtime-none,drop-does-block,no-pin
cargo test --release
cargo test --release --no-default-features --features runtime-none,drop-does-block
cargo test --release --features no-pin
cargo test --release --no-default-features --features runtime-none,drop-does-block,no-pin
*/



#[cfg(feature = "runtime-none")]
#[test]
fn basic_test() {
	use std::{pin::pin, thread, time::Duration};
	let data = String::from("Test Data");
	{
		make_type_connector!(RefString *1 = <'a> String);
		let scoped_data = ScopedRef::<RefString>::new(&data);
		let scoped_data = pin!(scoped_data);
		
		let data_ref = scoped_data.new_ref();
		thread::spawn(move || {
			println!("Sleeping for 1 second...");
			thread::sleep(Duration::from_secs(1));
			println!("Data: {data_ref}");
		});
	}
	
	println!("All threads finished!");
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn basic_test() {
	use std::{pin::pin, thread, time::Duration};
	let data = String::from("Test Data");
	{
		make_type_connector!(RefString *1 = <'a> String);
		let scoped_data = ScopedRef::<RefString>::new(&data);
		let scoped_data = pin!(scoped_data);
		
		let data_ref = scoped_data.new_ref();
		thread::spawn(move || {
			println!("Sleeping for 1 second...");
			thread::sleep(Duration::from_secs(1));
			println!("Data: {data_ref}");
		});
	}
	
	println!("All threads finished!");
}

#[cfg(feature = "runtime-none")]
#[test]
fn advanced_type_test() {
	use std::{thread, pin::pin, time::Duration};
	struct AdvancedType<'a> {
		inner: &'a u8,
	}
	let inner = 128;
	let data = AdvancedType {
		inner: &inner,
	};
	{
		make_type_connector!(RefAdvancedType *1 = <'a> AdvancedType<'a>);
		let scoped_data = ScopedRef::<RefAdvancedType>::new(&data);
		let scoped_data = pin!(scoped_data);
		
		let data_ref = scoped_data.new_ref();
		thread::spawn(move || {
			println!("Sleeping for 1 second...");
			thread::sleep(Duration::from_secs(1));
			println!("Data: {}", data_ref.inner().inner);
		});
	}
	
	println!("All threads finished!");
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn advanced_type_test() {
	use std::{pin::pin, thread, time::Duration};
	struct AdvancedType<'a> {
		inner: &'a u8,
	}
	let inner = 128;
	let data = AdvancedType {
		inner: &inner,
	};
	{
		make_type_connector!(RefAdvancedType *1 = <'a> AdvancedType<'a>);
		let scoped_data = ScopedRef::<RefAdvancedType>::new(&data);
		let scoped_data = pin!(scoped_data);
		
		let data_ref = scoped_data.new_ref();
		thread::spawn(move || {
			println!("Sleeping for 1 second...");
			thread::sleep(Duration::from_secs(1));
			println!("Data: {}", data_ref.inner().inner);
		});
	}
	
	println!("All threads finished!");
}

#[cfg(feature = "runtime-none")]
#[test]
fn test_macro() {
	
	make_type_connector!(MyType *5 = <'a> Vec<&'a u8>);
	
	let inner_data = 0u8;
	let _: <MyType as TypeConnector>::Super<'_> = vec!(&inner_data);
	let _: <MyType as TypeConnector>::RawPointerStorage = [0; 5];
	let _: <MyType as TypeConnector>::RawPointerStorage = <MyType as TypeConnector>::RAW_POINTER_DEFAULT;
	
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_macro() {
	
	make_type_connector!(MyType *5 = <'a> Vec<&'a u8>);
	
	let inner_data = 0u8;
	let _: <MyType as TypeConnector>::Super<'_> = vec!(&inner_data);
	let _: <MyType as TypeConnector>::RawPointerStorage = [0; 5];
	let _: <MyType as TypeConnector>::RawPointerStorage = <MyType as TypeConnector>::RAW_POINTER_DEFAULT;
	
}

#[cfg(feature = "runtime-none")]
#[test]
fn test_std_traits() {
	#[cfg(not(feature = "no-pin"))]
	use std::sync::atomic::Ordering;
	#[cfg(feature = "no-pin")]
	use std::sync::Arc;
	
	make_type_connector!(SliceU8 = <'a> [u8]);
	let data = vec!(1, 2, 3);
	let scoped_data = ScopedRef::<SliceU8>::new(&*data);
	#[cfg(not(feature = "no-pin"))]
	let scoped_data = std::pin::pin!(scoped_data);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref:?}"), String::from("[1, 2, 3]"));
	
	make_type_connector!(U8 = <'a> u8);
	let data = 123;
	let scoped_data = ScopedRef::<U8>::new(&data);
	#[cfg(not(feature = "no-pin"))]
	let scoped_data = std::pin::pin!(scoped_data);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref}"), String::from("123"));
	
	let data_ref_2 = data_ref.clone();
	#[cfg(not(feature = "no-pin"))]
	assert_eq!(scoped_data.counter_notify.0.load(Ordering::Acquire), 2);
	#[cfg(feature = "no-pin")]
	assert_eq!(Arc::strong_count(&scoped_data.counter_notify.0), 3);
	drop(data_ref);
	drop(data_ref_2);
	
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_std_traits() {
	#[cfg(not(feature = "no-pin"))]
	use std::sync::atomic::Ordering;
	#[cfg(all(feature = "no-pin", feature = "runtime-tokio"))]
	use std::sync::Arc;
	
	make_type_connector!(SliceU8 = <'a> [u8]);
	let data = vec!(1, 2, 3);
	let scoped_data = ScopedRef::<SliceU8>::new(&*data);
	#[cfg(not(feature = "no-pin"))]
	let scoped_data = std::pin::pin!(scoped_data);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref:?}"), String::from("[1, 2, 3]"));
	
	make_type_connector!(U8 = <'a> u8);
	let data = 123;
	let scoped_data = ScopedRef::<U8>::new(&data);
	#[cfg(not(feature = "no-pin"))]
	let scoped_data = std::pin::pin!(scoped_data);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref}"), String::from("123"));
	
	let data_ref_2 = data_ref.clone();
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-none"  ))]
	assert_eq!(scoped_data.counter_notify.0.load(Ordering::Acquire), 2);
	#[cfg(all(    feature = "no-pin" , feature = "runtime-none"  ))]
	assert_eq!(Arc::strong_count(&scoped_data.counter_notify), 3);
	#[cfg(all(not(feature = "no-pin"), feature = "runtime-tokio"))]
	assert_eq!(scoped_data.counter_notify.0.load(Ordering::Acquire), 2);
	#[cfg(all(    feature = "no-pin" , feature = "runtime-tokio"))]
	assert_eq!(Arc::strong_count(&scoped_data.counter_notify), 3);
	drop(data_ref);
	drop(data_ref_2);
	
}
