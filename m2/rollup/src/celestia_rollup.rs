#![deny(missing_docs)]
//! StarterRollup provides a minimal self-contained rollup implementation

use async_trait::async_trait;
use m2_stf::Runtime;
use sov_celestia_adapter::types::Namespace;
use sov_celestia_adapter::verifier::address::CelestiaAddress;
use sov_celestia_adapter::verifier::{CelestiaSpec, CelestiaVerifier, RollupParams};
use sov_celestia_adapter::{CelestiaConfig, CelestiaService};
use sov_db::sequencer_db::SequencerDB;
use sov_modules_api::default_spec::{DefaultSpec, ZkDefaultSpec};
use sov_modules_api::Spec;
use sov_modules_rollup_blueprint::RollupBlueprint;
use sov_modules_stf_blueprint::kernels::basic::BasicKernel;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_prover_storage_manager::ProverStorageManager;
use sov_risc0_adapter::host::Risc0Host;
use sov_risc0_adapter::Risc0Verifier;
use sov_rollup_interface::zk::aggregated_proof::CodeCommitment;
use sov_rollup_interface::zk::{ZkvmGuest, ZkvmHost};
use sov_state::config::Config as StorageConfig;
use sov_state::Storage;
use sov_state::{DefaultStorageSpec, ZkStorage};
use sov_stf_runner::ParallelProverService;
use sov_stf_runner::RollupConfig;
use sov_stf_runner::RollupProverConfig;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

/// The namespace for the rollup on Celestia.
const ROLLUP_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-celest");

/// The rollup stores the zk proofs in the namespace b"sov-test-p" on Celestia.
const ROLLUP_PROOF_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-p");

/// Rollup with [`CelestiaDaService`].
pub struct CelestiaRollup {}

/// This is the place, where all the rollup components come together and
/// they can be easily swapped with alternative implementations as needed.
#[async_trait]
impl RollupBlueprint for CelestiaRollup {
    type DaService = CelestiaService;
    type DaSpec = CelestiaSpec;
    type DaConfig = CelestiaConfig;
    type Vm = Risc0Host<'static>;

    /// Spec for the Zero Knowledge environment.
    type ZkSpec = ZkDefaultSpec<Risc0Verifier>;

    /// Spec for the Native environment.
    type NativeSpec = DefaultSpec<Risc0Verifier>;

    type StorageManager = ProverStorageManager<CelestiaSpec, DefaultStorageSpec>;
    type ZkRuntime = Runtime<Self::ZkSpec, Self::DaSpec>;

    type NativeRuntime = Runtime<Self::NativeSpec, Self::DaSpec>;

    type NativeKernel = BasicKernel<Self::NativeSpec, Self::DaSpec>;
    type ZkKernel = BasicKernel<Self::ZkSpec, Self::DaSpec>;

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

    fn create_rpc_methods(
        &self,
        storage: Arc<RwLock<<Self::NativeSpec as sov_modules_api::Spec>::Storage>>,
        ledger_db: &sov_db::ledger_db::LedgerDB,
        sequencer_db: &SequencerDB,
        da_service: &Self::DaService,
    ) -> Result<jsonrpsee::RpcModule<()>, anyhow::Error> {
        // TODO set the sequencer address
        let sequencer =
            CelestiaAddress::from_str("celestia1a68m2l85zn5xh0l07clk4rfvnezhywc53g8x7s")?;

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
        CelestiaService::new(
            rollup_config.da.clone(),
            RollupParams {
                rollup_batch_namespace: ROLLUP_NAMESPACE,
                rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
            },
        )
        .await
    }

    async fn create_prover_service(
        &self,
        prover_config: RollupProverConfig,
        rollup_config: &RollupConfig<Self::DaConfig>,
        _da_service: &Self::DaService,
    ) -> Self::ProverService {
        let vm = Risc0Host::new(m2_risc0::ROLLUP_ELF);
        let zk_stf = StfBlueprint::new();
        let zk_storage = ZkStorage::new();

        let da_verifier = CelestiaVerifier {
            rollup_namespace: ROLLUP_NAMESPACE,
        };

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
        rollup_config: &sov_stf_runner::RollupConfig<Self::DaConfig>,
    ) -> Result<Self::StorageManager, anyhow::Error> {
        let storage_config = StorageConfig {
            path: rollup_config.storage.path.clone(),
        };
        ProverStorageManager::new(storage_config)
    }
}

impl sov_modules_rollup_blueprint::WalletBlueprint for CelestiaRollup {}
