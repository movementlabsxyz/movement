use std::{
	collections::HashSet,
	fs,
	io::{BufWriter, Write},
	io::{Error, ErrorKind, Result},
	path::Path,
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

/// The whitelist file
pub type WhitelistFile = Arc<RwLock<HashSet<VerifyingKey>>>;

impl Whitelist {
	fn load(path: impl AsRef<Path>) -> Result<HashSet<VerifyingKey>> {
		let path: &Path = path.as_ref();
		if !path.exists() {
			fs::File::create(&path)?;
			tracing::info!("Create an empty Whitelist file: {:?}", path);
		}
		let content = fs::read_to_string(path)?;

		let mut set = HashSet::new();

		for line in content.lines() {
			let trimmed = line.trim().trim_start_matches("0x");
			if trimmed.is_empty() {
				continue;
			}

			let key_bytes = <[u8; 32]>::from_hex(trimmed)
				.map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid hex"))?;

			let verifying_key = VerifyingKey::from_bytes(&key_bytes)
				.map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid key"))?;

			set.insert(verifying_key);
		}

		Ok(set)
	}

	fn start_reload_thread(
		inner: WhitelistFile,
		path: impl AsRef<Path> + std::marker::Send + 'static,
	) {
		thread::spawn(move || loop {
			thread::sleep(Duration::from_secs(60));
			match Self::load(&path) {
				Ok(updated) => {
					if let Ok(mut guard) = inner.write() {
						*guard = updated;
					} else {
						tracing::error!("[whitelist] Failed to acquire write lock");
					}
				}
				Err(err) => {
					tracing::error!("[whitelist] Reload failed: {}", err);
				}
			}
		});
	}

	pub fn new(path: impl AsRef<Path> + std::marker::Send + 'static) -> Arc<RwLock<Self>> {
		let set = Self::load(&path).unwrap_or_default();
		let inner = Arc::new(RwLock::new(set));
		let arc_inner = inner.clone();
		Self::start_reload_thread(arc_inner, path);
		Arc::new(RwLock::new(Self { inner }))
	}

	pub fn contains(&self, key: &VerifyingKey) -> bool {
		self.inner.read().unwrap().contains(key)
	}

	pub fn from_file_and_spawn_reload_thread(
		path: impl AsRef<Path> + std::marker::Send + 'static,
	) -> Result<Self> {
		let set = Self::load(&path)?;
		let inner = Arc::new(RwLock::new(set));
		Self::start_reload_thread(inner.clone(), path);
		Ok(Self { inner })
	}

	pub fn save(
		path: impl AsRef<Path> + std::marker::Copy,
		hex_strings: &[VerifyingKey],
	) -> Result<()> {
		let mut current_list = Whitelist::load(path)?;
		hex_strings.into_iter().for_each(|pk| {
			current_list.insert(*pk);
		});
		let to_save = current_list
			.into_iter()
			.map(|pk| format!("0x{}", hex::encode(pk.to_bytes())))
			.collect::<Vec<String>>();

		let file = fs::File::create(path)?;
		let mut writer = BufWriter::new(file);

		for hex in to_save {
			writeln!(writer, "{}", hex)?;
		}
		writer.flush()?; // Ensure all data is written
		Ok(())
	}
}
