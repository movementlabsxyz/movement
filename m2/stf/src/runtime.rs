#![allow(unused_doc_comments)]
//! This module implements the core logic of the rollup.
//! To add new functionality to your rollup:
//!   1. Add a new module dependency to your `Cargo.toml` file
//!   2. Add the module to the `Runtime` below
//!   3. Update `genesis.json` with any additional data required by your new module

#[cfg(feature = "native")]
pub use sov_accounts::{AccountsRpcImpl, AccountsRpcServer};
#[cfg(feature = "native")]
pub use sov_bank::{BankRpcImpl, BankRpcServer};
use sov_modules_api::{macros::DefaultRuntime, Event};
use sov_modules_api::{Spec, DaSpec, DispatchCall, Genesis, MessageCodec};
#[cfg(feature = "native")]
pub use sov_sequencer_registry::{SequencerRegistryRpcImpl, SequencerRegistryRpcServer};

#[cfg(feature = "native")]
use crate::genesis_config::GenesisPaths;

/// The runtime defines the logic of the rollup.
///
/// At a high level, the rollup node receives serialized "call messages" from the DA layer and executes them as atomic transactions.
/// Upon reception, the message is deserialized and forwarded to an appropriate module.
///
/// The module-specific logic is implemented by module creators, but all the glue code responsible for message
/// deserialization/forwarding is handled by a rollup `runtime`.
///
/// In order to define the runtime we need to specify all the modules supported by our rollup (see the `Runtime` struct bellow)
///
/// The `Runtime` defines:
/// - how the rollup modules are wired up together.
/// - how the state of the rollup is initialized.
/// - how messages are dispatched to appropriate modules.
///
/// Runtime lifecycle:
///
/// 1. Initialization:
///     When a rollup is deployed for the first time, it needs to set its genesis state.
///     The `#[derive(Genesis)` macro will generate `Runtime::genesis(config)` method which returns
///     `Storage` with the initialized state.
///
/// 2. Calls:      
///     The `Module` interface defines a `call` method which accepts a module-defined type and triggers the specific `module logic.`
///     In general, the point of a call is to change the module state, but if the call throws an error,
///     no state is updated (the transaction is reverted).
///
/// `#[derive(MessageCodec)` adds deserialization capabilities to the `Runtime` (by implementing the `decode_call` method).
/// `Runtime::decode_call` accepts a serialized call message and returns a type that implements the `DispatchCall` trait.
///  The `DispatchCall` implementation (derived by a macro) forwards the message to the appropriate module and executes its `call` method.
#[cfg_attr(
    feature = "native",
    derive(sov_modules_api::macros::CliWallet),
    sov_modules_api::macros::expose_rpc
)]
#[derive(Genesis, DispatchCall, Event, MessageCodec, DefaultRuntime)]
#[serialization(
borsh::BorshDeserialize,
borsh::BorshSerialize,
serde::Serialize,
serde::Deserialize
)]
pub struct Runtime<S: Spec, Da: DaSpec> {
    /// The `accounts` module is responsible for managing user accounts and their nonces
    pub accounts: sov_accounts::Accounts<S>,
    /// The bank module is responsible for minting, transferring, and burning tokens
    pub bank: sov_bank::Bank<S>,
    /// The sequencer registry module is responsible for authorizing users to sequencer rollup transactions
    pub sequencer_registry: sov_sequencer_registry::SequencerRegistry<S, Da>,
}

impl<S, Da> sov_modules_stf_blueprint::Runtime<S, Da> for Runtime<S, Da>
where
    S: Spec,
    Da: DaSpec,
{
    type GenesisConfig = GenesisConfig<S, Da>;

    #[cfg(feature = "native")]
    type GenesisPaths = GenesisPaths;

    #[cfg(feature = "native")]
    fn rpc_methods(
        storage: std::sync::Arc<std::sync::RwLock<<S as Spec>::Storage>>,
    ) -> jsonrpsee::RpcModule<()> {
        get_rpc_methods::<S, Da>(storage)
    }

    #[cfg(feature = "native")]
    fn genesis_config(
        genesis_paths: &Self::GenesisPaths,
    ) -> Result<Self::GenesisConfig, anyhow::Error> {
        crate::genesis_config::get_genesis_config(genesis_paths)
    }
}