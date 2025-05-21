use aptos_sdk::types::{
	account_address::{create_resource_address, AccountAddress},
	LocalAccount,
};
use rand::thread_rng;
use rand::Rng;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc, Mutex,
};
use std::time::Instant;

fn matches(addr: &AccountAddress, start: &str, end: &str) -> bool {
	let addr_hex = hex::encode(addr);

	(start.is_empty() || addr_hex.starts_with(start)) && (end.is_empty() || addr_hex.ends_with(end))
}

pub fn mine_move_address(starts: &str, ends: &str, threads: usize) -> LocalAccount {
	let found = Arc::new(AtomicBool::new(false));
	let result = Arc::new(Mutex::new(None));
	let start_time = Instant::now();

	let pool = ThreadPoolBuilder::new()
		.num_threads(threads)
		.build()
		.expect("Failed to build thread pool");

	pool.install(|| {
		(0u64..=u64::MAX).into_par_iter().find_any(|_| {
			if found.load(Ordering::Relaxed) {
				return false;
			}
			let mut rng = thread_rng();
			let account = LocalAccount::generate(&mut rng);
			if matches(&account.address(), starts, ends) {
				let mut lock = result.lock().unwrap();
				*lock = Some(account);
				found.store(true, Ordering::Relaxed);
				true
			} else {
				false
			}
		});
	});

	println!("Mining completed in {:?}", start_time.elapsed());

	let final_result = std::mem::take(&mut *result.lock().unwrap());
	final_result.expect("No matching account found")
}

pub fn mine_resource_address(
	address: &AccountAddress,
	starts: &str,
	ends: &str,
	threads: usize,
) -> (AccountAddress, Vec<u8>) {
	let found = Arc::new(AtomicBool::new(false));
	let result = Arc::new(Mutex::new(None));
	let start_time = Instant::now();

	let pool = ThreadPoolBuilder::new()
		.num_threads(threads)
		.build()
		.expect("Failed to build thread pool");

	pool.install(|| {
		(0u64..=u64::MAX).into_par_iter().find_any(|_| {
			if found.load(Ordering::Relaxed) {
				return false;
			}
			let mut rng = thread_rng();
			let mut seed_bytes = [0u8; 32]; // or any size you need
			rng.fill(&mut seed_bytes);
			let resource_address = create_resource_address(*address, &seed_bytes);
			if matches(&resource_address, starts, ends) {
				let mut lock = result.lock().unwrap();
				*lock = Some((resource_address, seed_bytes.to_vec()));
				found.store(true, Ordering::Relaxed);
				true
			} else {
				false
			}
		});
	});

	println!("Mining completed in {:?}", start_time.elapsed());

	let final_result = std::mem::take(&mut *result.lock().unwrap());
	final_result.expect("No matching account found")
}
