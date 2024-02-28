use anyhow::{bail, Context as _};
use sov_modules_api::{Context, DaSpec};
use sov_modules_stf_blueprint::Runtime as RuntimeTrait;
use std::path::{Path, PathBuf};

use crate::Runtime;

/// Paths to genesis files
pub struct GenesisPaths {
    /// Accounts genesis path
    pub accounts_genesis_path: PathBuf,
    /// Bank genesis path
    pub bank_genesis_path: PathBuf,
    /// Sequence genesis path
    pub sequence_genesis_path: PathBuf,
}

impl GenesisPaths {
    /// Creates a new [`GenesisPaths`] from the files contained in the given
    /// directory.
    ///
    /// Take a look at the contents of the `test_data` directory to see the
    /// expected files.
    pub fn from_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            accounts_genesis_path: dir.as_ref().join("accounts_genesis.json"),
            bank_genesis_path: dir.as_ref().join("bank_genesis.json"),
            sequence_genesis_path: dir.as_ref().join("sequence_genesis.json"),
        }
    }
}

/// Creates genesis configuration.
pub(crate) fn get_genesis_config<C: Context, Da: DaSpec>(
    genesis_paths: &GenesisPaths,
) -> Result<<Runtime<C, Da> as RuntimeTrait<C, Da>>::GenesisConfig, anyhow::Error> {
}
