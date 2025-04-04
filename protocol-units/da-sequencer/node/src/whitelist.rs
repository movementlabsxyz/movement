use std::{
	collections::HashSet,
	fs,
	path::PathBuf,
	sync::{Arc, RwLock},
	thread,
	time::Duration,
};

use ed25519_dalek::VerifyingKey;
use hex::FromHex;

#[derive(Clone)]
pub struct Whitelist {
	pub(crate) inner: Arc<RwLock<HashSet<VerifyingKey>>>,
}

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
						eprintln!("[whitelist] Failed to acquire write lock");
					}
				}
				Err(err) => {
					eprintln!("[whitelist] Reload failed: {}", err);
				}
			}
		});
	}

	pub fn new(path: PathBuf) -> Arc<RwLock<Self>> {
		let set = Self::load(&path).unwrap_or_default();
		let inner = Arc::new(RwLock::new(set));
		let arc_inner = inner.clone();
		Self::start_reload_thread(arc_inner, path);
		Arc::new(RwLock::new(Self { inner }))
	}

	pub fn contains(&self, key: &VerifyingKey) -> bool {
		self.inner.read().unwrap().contains(key)
	}

	pub fn from_file_and_spawn_reload_thread(path: PathBuf) -> std::io::Result<Self> {
		let set = Self::load(&path)?;
		let inner = Arc::new(RwLock::new(set));
		Self::start_reload_thread(inner.clone(), path);
		Ok(Self { inner })
	}
}
