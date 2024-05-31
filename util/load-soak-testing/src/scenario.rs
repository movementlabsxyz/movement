use crate::TestExecutionError;
use crate::EXEC_LOG_FILTER;

<<<<<<< HEAD
/// A scenario is any struct that implements the Scenario trait.
/// To ease scenario execution login and using id (usize) is provided during creation.
/// If you use it, it can be return after the scenario execution otherwise return any usize.
/// This id is only use to identify scenario execution is the logs.

/// Implements this trait to develop a scenario.
/// Scenario execution logs:
///  * on std out using tracing::{error, warn, info, debug, trace}
///  * the same logs goes in the log file defined in the config.
///  * to log in the execution json formated file use the log_exec_info function opf the trait.
///
///  Return the execution result (Result<(), TestExecutionError>) and a usize, id of the scenario or any usize.
#[async_trait::async_trait]
pub trait Scenario {
	async fn run(self: Box<Self>) -> (usize, Result<(), TestExecutionError>);

	fn log_exec_info(&self, msg: &str) {
		tracing::info!(target:EXEC_LOG_FILTER, msg);
	}
}

/// Type definition that is used by the test executor to create scenario to execute.
pub type CreateScenarioFn = (dyn Fn(usize) -> Box<dyn Scenario> + Send + Sync);
=======
pub struct Scenario {
	id: usize,
}

impl Scenario {
	pub fn new(id: usize) -> Self {
		Scenario { id }
	}

	pub async fn run(self) -> (usize, Result<(), TestExecutionError>) {
		tracing::info!("Scenarios:{} start", self.id);
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		Self::log_exec_result(&format!("Scenario:{} ended", self.id));
		(self.id, Ok(()))
	}

	fn log_exec_result(msg: &str) {
		tracing::info!(target:EXEC_LOG_FILTER, msg);
	}
}
>>>>>>> ff595882 (first version of the scenario executor.)
