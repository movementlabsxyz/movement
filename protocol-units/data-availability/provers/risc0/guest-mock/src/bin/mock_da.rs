#![no_main]
//! This binary implements the verification logic for the rollup. This is the code that runs inside
//! of the zkvm in order to generate proofs for the rollup.

use sov_mock_da::MockDaVerifier;
use sov_modules_api::default_context::ZkDefaultContext;
use sov_modules_stf_blueprint::kernels::basic::BasicKernel;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_risc0_adapter::guest::Risc0Guest;
use sov_state::ZkStorage;
use stf_starter::runtime::Runtime;
use stf_starter::StfVerifier;

risk0_zkvm::guest::entry!(main);

pub fn main() {
    let guest = Risc0Guest::new();
    let storage = ZkStorage::new();
    #[cfg(feature = "bench")]
    let start_cycles = env::get_cycle_count();

    let stf: StfBlueprint<ZkDefaultContext, _, _, Runtime<_, _>> = StfBlueprint::new();
    let stf_verifier = StfVerifier::new(stf, MockDaVerifier {});

    stf_verifier
        .run_block(guest, storage)
        .expect("Prover must be honest");
}
