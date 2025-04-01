use std::{
	collections::HashSet,
	fs,
	path::PathBuf,
	sync::{Arc, Mutex, RwLock},
	thread,
	time::Duration,
};

use ed25519_dalek::VerifyingKey;
use hex::FromHex;
use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct Whitelist {
	inner: Arc<RwLock<HashSet<VerifyingKey>>>,
}

pub static INSTANCE: Lazy<Mutex<Whitelist>> =
	Lazy::new(|| Mutex::new(Whitelist { inner: Arc::new(RwLock::new(HashSet::new())) }));

impl Whitelist {
	fn load(path: &PathBuf) -> std::io::Result<HashSet<VerifyingKey>> {
		let content = fs::read_to_string(path)?;
		let mut set = HashSet::new();

		for line in content.lines() {
			let trimmed = line.trim().trim_start_matches("0x");
			if trimmed.is_empty() {
				continue;
			}

			let key_bytes = <[u8; 32]>::from_hex(trimmed)
				.map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid hex"))?;

			let verifying_key = VerifyingKey::from_bytes(&key_bytes)
				.map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key"))?;

			set.insert(verifying_key);
		}

		Ok(set)
	}

	fn start_reload_thread(inner: Arc<RwLock<HashSet<VerifyingKey>>>, path: PathBuf) {
		thread::spawn(move || loop {
			thread::sleep(Duration::from_secs(60));
			match Self::load(&path) {
				Ok(updated) => {
					if let Ok(mut guard) = inner.write() {
						*guard = updated;
					} else {
						tracing::warn!("[whitelist] Failed to acquire write lock");
					}
				}
				Err(err) => {
					tracing::warn!("[whitelist] Reload failed: {}", err);
				}
			}
		});
	}

	pub fn init_global(path: PathBuf) {
		let set = Self::load(&path).unwrap_or_default();
		let inner = Arc::new(RwLock::new(set));
		Self::start_reload_thread(inner.clone(), path);

		let mut instance = INSTANCE.lock().unwrap();
		*instance = Self { inner };
	}

	pub fn contains(&self, key: &VerifyingKey) -> bool {
		self.inner.read().unwrap().contains(key)
	}

	/// Returns a locked reference to the global whitelist.
	pub fn get<'a>() -> std::sync::MutexGuard<'a, Whitelist> {
		INSTANCE.lock().unwrap()
	}

	#[cfg(test)]
	pub fn from_keys(keys: Vec<VerifyingKey>) -> Self {
		let set = keys.into_iter().collect::<HashSet<_>>();
		Self { inner: Arc::new(RwLock::new(set)) }
	}

	#[cfg(test)]
	pub fn set_keys(&mut self, keys: Vec<VerifyingKey>) {
		let new_set: HashSet<_> = keys.into_iter().collect();
		*self.inner.write().unwrap() = new_set;
	}

	#[cfg(test)]
	pub fn clear(&mut self) {
		self.inner.write().unwrap().clear();
	}

	#[cfg(test)]
	pub fn insert(&mut self, key: VerifyingKey) {
		self.inner.write().unwrap().insert(key);
	}
}
