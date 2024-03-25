#![deny(missing_docs)]
//! StarterRollup provides a minimal self-contained rollup implementation

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use sov_db::ledger_db::LedgerDB;
use sov_db::sequencer_db::SequencerDB;
use sov_mock_da::{MockAddress, MockDaConfig, MockDaService, MockDaSpec};
use sov_modules_api::default_spec::{DefaultSpec, ZkDefaultSpec};
use sov_modules_api::Spec;
use sov_modules_rollup_blueprint::RollupBlueprint;
use sov_modules_stf_blueprint::kernels::basic::BasicKernel;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_prover_storage_manager::ProverStorageManager;
use sov_risc0_adapter::host::Risc0Host;
use sov_risc0_adapter::Risc0Verifier;
use sov_rollup_interface::zk::{aggregated_proof::CodeCommitment, ZkvmGuest, ZkvmHost};
use sov_state::config::Config as StorageConfig;
use sov_state::Storage;
use sov_state::{DefaultStorageSpec, ZkStorage};
use sov_stf_runner::ParallelProverService;
use sov_stf_runner::RollupConfig;
use sov_stf_runner::RollupProverConfig;
use m2_stf::Runtime;

/// Rollup with [`MockDaService`].
pub struct MockRollup {}

/// This is the place, where all the rollup components come together and
/// they can be easily swapped with alternative implementations as needed.
#[async_trait]
impl RollupBlueprint for MockRollup {
    /// This component defines the Data Availability layer.
    type DaService = MockDaService;
    type DaSpec = MockDaSpec;
    type DaConfig = MockDaConfig;

    /// The concrete ZkVm used in the rollup.
    type Vm = Risc0Host<'static>;

    /// Spec for the Zero Knowledge environment.
    type ZkSpec = ZkDefaultSpec<Risc0Verifier>;

    /// Spec for the Native environment.
    type NativeSpec = DefaultSpec<Risc0Verifier>;

    /// Manager for the native storage lifecycle.
    type StorageManager = ProverStorageManager<MockDaSpec, DefaultStorageSpec>;

    /// Runtime for the Zero Knowledge environment.
    type ZkRuntime = Runtime<Self::ZkSpec, Self::DaSpec>;
    /// Runtime for the Native environment.
    type NativeRuntime = Runtime<Self::NativeSpec, Self::DaSpec>;

    /// Kernels.
    type NativeKernel = BasicKernel<Self::NativeSpec, Self::DaSpec>;
    type ZkKernel = BasicKernel<Self::ZkSpec, Self::DaSpec>;

    /// Prover service.
    type ProverService = ParallelProverService<
        <<Self::NativeSpec as Spec>::Storage as Storage>::Root,
        <<Self::NativeSpec as Spec>::Storage as Storage>::Witness,
        Self::DaService,
        Self::Vm,
        StfBlueprint<
            Self::ZkSpec,
            Self::DaSpec,
            <<Self::Vm as ZkvmHost>::Guest as ZkvmGuest>::Verifier,
            Self::ZkRuntime,
            Self::ZkKernel,
        >,
    >;

    /// This function generates RPC methods for the rollup, allowing for extension with custom endpoints.
    fn create_rpc_methods(
        &self,
        storage: Arc<RwLock<<Self::NativeSpec as Spec>::Storage>>,
        ledger_db: &LedgerDB,
        sequencer_db: &SequencerDB,
        da_service: &Self::DaService,
    ) -> Result<jsonrpsee::RpcModule<()>, anyhow::Error> {
        // TODO set the sequencer address
        let sequencer = MockAddress::new([0; 32]);

        #[allow(unused_mut)]
        let mut rpc_methods = sov_modules_rollup_blueprint::register_rpc::<
            Self::NativeRuntime,
            Self::NativeSpec,
            Self::DaService,
        >(storage, ledger_db, sequencer_db, da_service, sequencer)?;

        #[cfg(feature = "experimental")]
        crate::eth::register_ethereum::<Self::DaService>(
            da_service.clone(),
            storage.clone(),
            &mut rpc_methods,
        )?;

        Ok(rpc_methods)
    }

    async fn create_da_service(
        &self,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> Self::DaService {
        MockDaService::new(rollup_config.da.sender_address)
    }

    async fn create_prover_service(
        &self,
        prover_config: RollupProverConfig,
        rollup_config: &RollupConfig<Self::DaConfig>,
        _da_service: &Self::DaService,
    ) -> Self::ProverService {
        let vm = Risc0Host::new(m2_risc0::MOCK_DA_ELF);
        let zk_stf = StfBlueprint::new();
        let zk_storage = ZkStorage::new();
        let da_verifier = Default::default();

        ParallelProverService::new_with_default_workers(
            vm,
            zk_stf,
            da_verifier,
            prover_config,
            zk_storage,
            rollup_config.prover_service,
            CodeCommitment::default(),
        )
    }

    fn create_storage_manager(
        &self,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> anyhow::Result<Self::StorageManager> {
        let storage_config = StorageConfig {
            path: rollup_config.storage.path.clone(),
        };
        ProverStorageManager::new(storage_config)
    }
}

impl sov_modules_rollup_blueprint::WalletBlueprint for MockRollup {}
