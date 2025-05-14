use aptos_sdk::types::{account_address::AccountAddress, LocalAccount};
use rand::thread_rng;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;
use tracing;

fn matches_pattern(addr: &AccountAddress, start: &[u8], end: &[u8]) -> bool {
    let addr_bytes = addr.to_vec();
    (start.is_empty() || addr_bytes.starts_with(start)) &&
    (end.is_empty() || addr_bytes.ends_with(end))
}

pub fn mine_move_address(
    starts_pattern: &[u8],
    ends_pattern: &[u8],
    threads: usize,
) -> LocalAccount {
    let found = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));
    let start_time = Instant::now();

    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to build thread pool");

    pool.install(|| {
        (0u64..=u64::MAX)
            .into_par_iter()
            .find_any(|_| {
                if found.load(Ordering::Relaxed) {
                    return false;
                }
                let mut rng = thread_rng();
                let account = LocalAccount::generate(&mut rng);
                if matches_pattern(&account.address(), starts_pattern, ends_pattern) {
                    let mut lock = result.lock().unwrap();
                    *lock = Some(account);
                    found.store(true, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            });
    });

    tracing::info!("Mining completed in {:?}", start_time.elapsed());

    let final_result = std::mem::take(&mut *result.lock().unwrap());
    final_result.expect("No matching account found")
}
