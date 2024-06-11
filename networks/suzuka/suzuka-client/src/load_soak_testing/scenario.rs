use super::EXEC_LOG_FILTER;
<<<<<<< HEAD

/// A scenario is any struct that implements the Scenario trait.
/// To ease scenario execution and logs, an id (usize) is provided during creation.
/// This id is only used by the runtime to identify scenario execution is the logs.
=======
use anyhow::Result;

/// A scenario is any struct that implements the Scenario trait.
/// To ease scenario execution and logs, an id (usize) is provided during creation.
/// This id is only used by teh runtime to identify scenario execution is the logs.
>>>>>>> 186a4994 (recreate the PR to remove unknown modifications)

/// Implements this trait to develop a scenario.
/// How logs works during scenario execution:
///  * log on stdout using tracing::{error, warn, info, debug, trace} macro.
///  * the same logs in a file defined in the config (logfile).
///  * to log in the execution json formatted file, use the log_exec_info function of the trait.
///
///  Return the execution result. If the scenario fails, return an error.
#[async_trait::async_trait]
pub trait Scenario {
<<<<<<< HEAD
	async fn run(self: Box<Self>) -> Result<(), anyhow::Error>;
=======
	async fn run(self: Box<Self>) -> Result<()>;
>>>>>>> 186a4994 (recreate the PR to remove unknown modifications)

	fn log_exec_info(&self, msg: &str) {
		tracing::info!(target:EXEC_LOG_FILTER, msg);
	}
}

/// Type definition that is used by the test executor to create scenario to execute.
pub type CreateScenarioFn = (dyn Fn(usize) -> Box<dyn Scenario> + Send + Sync);
