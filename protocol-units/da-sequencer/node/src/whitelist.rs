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
use once_cell::sync::OnceCell;

#[derive(Clone)]
pub struct Whitelist {
        inner: Arc<RwLock<HashSet<VerifyingKey>>>,
}

#[cfg(not(test))]
pub static INSTANCE: OnceCell<Whitelist> = OnceCell::new();

#[cfg(test)]
pub static INSTANCE: OnceCell<Whitelist> = OnceCell::new();

impl Whitelist {
        /// Loads public keys from a file. Each line must be a hex-encoded 32-byte public key.
        fn load(path: &PathBuf) -> std::io::Result<HashSet<VerifyingKey>> {
                let content = fs::read_to_string(path)?;
                let mut set = HashSet::new();

                for line in content.lines() {
                        let trimmed = line.trim().trim_start_matches("0x");
                        if trimmed.is_empty() {
                                continue;
                        }

                        let key_bytes = <[u8; 32]>::from_hex(trimmed).map_err(|_| {
                                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid hex")
                        })?;

                        let verifying_key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| {
                                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key")
                        })?;

                        set.insert(verifying_key);
                }

                Ok(set)
        }

        /// Starts a background thread to reload the whitelist from disk every 60 seconds.
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

        /// Initializes the global whitelist from the given file path.
        /// This must be called once at application startup.
        pub fn init_global(path: PathBuf) {
                let set = Self::load(&path).unwrap_or_default();
                let inner = Arc::new(RwLock::new(set));
                Self::start_reload_thread(inner.clone(), path);

                INSTANCE.set(Self { inner }).unwrap_or_else(|_| {
                        eprintln!("[whitelist] Global whitelist already initialized");
                });
        }

        /// Returns a reference to the global whitelist instance.
        /// Panics if `init_global` has not been called.
        pub fn get() -> &'static Whitelist {
                INSTANCE.get().expect("Whitelist not initialized")
        }

        /// Returns true if the given public key is in the whitelist.
        pub fn contains(&self, key: &VerifyingKey) -> bool {
                self.inner.read().unwrap().contains(key)
        }

        /// Creates a Whitelist from a list of keys, for use in tests.
        #[cfg(test)]
        pub fn from_keys(keys: Vec<VerifyingKey>) -> Self {
                let set = keys.into_iter().collect::<HashSet<_>>();
                Self { inner: Arc::new(RwLock::new(set)) }
        }
}
