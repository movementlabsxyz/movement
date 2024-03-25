use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _};
use sov_accounts::AccountConfig;
use sov_bank::BankConfig;
use sov_modules_api::{Spec, DaSpec};
use sov_modules_stf_blueprint::Runtime as RuntimeTrait;
use sov_sequencer_registry::SequencerConfig;
use sov_stf_runner::read_json_file;

use super::GenesisConfig;
use crate::Runtime;

/// Paths to genesis files.
pub struct GenesisPaths {
    /// Accounts genesis path.
    pub accounts_genesis_path: PathBuf,
    /// Bank genesis path.
    pub bank_genesis_path: PathBuf,
    /// Sequencer Registry genesis path.
    pub sequencer_genesis_path: PathBuf,
}

impl GenesisPaths {
    /// Creates a new [`GenesisPaths`] from the files contained in the given
    /// directory.
    ///
    /// Take a look at the contents of the `test_data` directory to see the
    /// expected files.
    pub fn from_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            accounts_genesis_path: dir.as_ref().join("accounts.json"),
            bank_genesis_path: dir.as_ref().join("bank.json"),
            sequencer_genesis_path: dir.as_ref().join("sequencer_registry.json"),
        }
    }
}

/// Creates genesis configuration.
pub(crate) fn get_genesis_config<S: Spec, Da: DaSpec>(
    genesis_paths: &GenesisPaths,
) -> Result<<Runtime<S, Da> as RuntimeTrait<S, Da>>::GenesisConfig, anyhow::Error> {
    let genesis_config =
        create_genesis_config(genesis_paths).context("Unable to read genesis configuration")?;

    validate_config(genesis_config)
}

fn validate_config<S: Spec, Da: DaSpec>(
    genesis_config: <Runtime<S, Da> as RuntimeTrait<S, Da>>::GenesisConfig,
) -> Result<<Runtime<S, Da> as RuntimeTrait<S, Da>>::GenesisConfig, anyhow::Error> {
    let token_address = &genesis_config.bank.tokens[0].token_address;


    let coins_token_addr = &genesis_config
        .sequencer_registry
        .coins_to_lock
        .token_address;

    if coins_token_addr != token_address {
        bail!(
            "Wrong token address in `sequencer_registry_config` expected {} but found {}",
            token_address,
            coins_token_addr
        )
    }

    Ok(genesis_config)
}

fn create_genesis_config<S: Spec, Da: DaSpec>(
    genesis_paths: &GenesisPaths,
) -> anyhow::Result<GenesisConfig<S, Da>> {
    let accounts_config: AccountConfig<S> = read_json_file(&genesis_paths.accounts_genesis_path)?;
    let bank_config: BankConfig<S> = read_json_file(&genesis_paths.bank_genesis_path)?;
    let sequencer_registry_config: SequencerConfig<S, Da> =
        read_json_file(&genesis_paths.sequencer_genesis_path)?;

    Ok(GenesisConfig::new(
        accounts_config,
        bank_config,
        sequencer_registry_config,
    ))
}