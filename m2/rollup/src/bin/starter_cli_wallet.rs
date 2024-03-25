//! This binary defines a cli wallet for interacting
//! with the rollup.

use m2_stf::runtime::RuntimeSubcommand;
use sov_modules_api::cli::{FileNameArg, JsonStringArg};
use sov_modules_rollup_blueprint::WalletBlueprint;
#[cfg(feature = "celestia_da")]
use sov_rollup_starter::celestia_rollup::CelestiaRollup as StarterRollup;
#[cfg(feature = "mock_da")]
use m2_rollup::mock_rollup::MockRollup as StarterRollup;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    StarterRollup::run_wallet::<
        RuntimeSubcommand<FileNameArg, _, _>,
        RuntimeSubcommand<JsonStringArg, _, _>,
    >()
    .await
}
