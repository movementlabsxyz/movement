use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
	let mut s = DefaultHasher::new();
	t.hash(&mut s);
	s.finish()
}

#[test]
fn test_calculate_hash() {
	let a = "hash";
	let b = "hash";
	let c = "other";

	assert_eq!(calculate_hash(&a), calculate_hash(&b));
	assert_ne!(calculate_hash(&a), calculate_hash(&c));
}
