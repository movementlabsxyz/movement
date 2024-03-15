//! The rollup State Transition Function.

#[cfg(feature = "native")]
pub mod genesis_config;
mod hooks;
pub mod runtime;
pub use runtime::*;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_rollup_interface::da::DaVerifier;
use sov_stf_runner::verifier::StateTransitionVerifier;

/// Alias for StateTransitionVerifier.
pub type StfVerifier<DA, Vm, ZkContext, RT, K> =
    StateTransitionVerifier<StfBlueprint<ZkContext, <DA as DaVerifier>::Spec, Vm, RT, K>, DA, Vm>;

pub use sov_mock_da::MockDaSpec;