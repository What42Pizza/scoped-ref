#[allow(unused_imports)] // idk why rust (sometimes) says this is unused
use crate::*;



/*
To test all situations, run:
cargo test --no-default-features --features drop-does-block,unwind-does-abort,runtime-none
cargo test --no-default-features --features drop-does-block,unwind-does-abort,runtime-tokio
cargo test --no-default-features --features drop-does-block,unwind-does-abort,runtime-none,no-pin
cargo test --no-default-features --features drop-does-block,unwind-does-abort,runtime-tokio,no-pin
cargo test --release --no-default-features --features drop-does-block,unwind-does-abort,runtime-none
cargo test --release --no-default-features --features drop-does-block,unwind-does-abort,runtime-tokio
cargo test --release --no-default-features --features drop-does-block,unwind-does-abort,runtime-none,no-pin
cargo test --release --no-default-features --features drop-does-block,unwind-does-abort,runtime-tokio,no-pin
*/



#[cfg(feature = "runtime-none")]
#[test]
fn basic_test() {
	use std::{thread, time::Duration};
	let data = String::from("Test Data");
	{
		make_type_connector!(RefString = <'a> String);
		make_scoped_ref!(scoped_data = (&data) as RefString);
		
		let data_ref = scoped_data.new_ref();
		thread::spawn(move || {
			println!("Sleeping for 0.1 seconds...");
			thread::sleep(Duration::from_millis(100));
			println!("Data: {data_ref}");
		});
	}
	
	println!("All threads finished!");
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn basic_test() {
	use std::{thread, time::Duration};
	let data = String::from("Test Data");
	{
		make_type_connector!(RefString = <'a> String);
		make_scoped_ref!(scoped_data = (&data) as RefString);
		
		let data_ref = scoped_data.new_ref();
		thread::spawn(move || {
			println!("Sleeping for 0.1 seconds...");
			thread::sleep(Duration::from_millis(100));
			println!("Data: {data_ref}");
		});
	}
	
	println!("All threads finished!");
}



#[cfg(feature = "runtime-none")]
#[test]
fn advanced_type_test() {
	struct AdvancedType<'a> {
		inner: &'a u8,
	}
	let inner = 128;
	let data = AdvancedType {
		inner: &inner,
	};
	{
		make_type_connector!(RefAdvancedType = <'a> AdvancedType<'a>);
		make_scoped_ref!(scoped_data = (&data) as RefAdvancedType);
		
		let data_ref = scoped_data.new_ref();
		std::thread::spawn(move || {
			println!("Data: {}", data_ref.inner().inner);
		});
	}
	
	println!("All threads finished!");
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn advanced_type_test() {
	struct AdvancedType<'a> {
		inner: &'a u8,
	}
	let inner = 128;
	let data = AdvancedType {
		inner: &inner,
	};
	{
		make_type_connector!(RefAdvancedType = <'a> AdvancedType<'a>);
		make_scoped_ref!(scoped_data = (&data) as RefAdvancedType);
		
		let data_ref = scoped_data.new_ref();
		std::thread::spawn(move || {
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
	
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_macro() {
	
	make_type_connector!(MyType = <'a> Vec<&'a u8>);
	
	let inner_data = 0u8;
	let _: <MyType as TypeConnector>::Super<'_> = vec!(&inner_data);
	
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
	make_scoped_ref!(scoped_data = (&*data) as SliceU8);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref:?}"), String::from("[1, 2, 3]"));
	
	make_type_connector!(U8 = <'a> u8);
	let data = 123;
	make_scoped_ref!(scoped_data = (&data) as U8);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref}"), String::from("123"));
	
	let data_ref_2 = data_ref.clone();
	#[cfg(not(feature = "no-pin"))]
	assert_eq!(scoped_data.counter_notify.0.load(Ordering::Acquire), 2);
	#[cfg(feature = "no-pin")]
	assert_eq!(Arc::strong_count(&scoped_data.counter_notify), 3);
	drop(data_ref);
	drop(data_ref_2);
	
}
#[cfg(feature = "runtime-tokio")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_std_traits() {
	#[cfg(not(feature = "no-pin"))]
	use std::sync::atomic::Ordering;
	#[cfg(feature = "no-pin")]
	use std::sync::Arc;
	
	make_type_connector!(SliceU8 = <'a> [u8]);
	let data = vec!(1, 2, 3);
	make_scoped_ref!(scoped_data = (&*data) as SliceU8);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref:?}"), String::from("[1, 2, 3]"));
	
	make_type_connector!(U8 = <'a> u8);
	let data = 123;
	make_scoped_ref!(scoped_data = (&data) as U8);
	let data_ref = scoped_data.new_ref();
	assert_eq!(format!("{data_ref}"), String::from("123"));
	
	let data_ref_2 = data_ref.clone();
	#[cfg(not(feature = "no-pin"))]
	assert_eq!(scoped_data.counter_notify.0.load(Ordering::Acquire), 2);
	#[cfg(feature = "no-pin")]
	assert_eq!(Arc::strong_count(&scoped_data.counter_notify), 3);
	drop(data_ref);
	drop(data_ref_2);
	
}
