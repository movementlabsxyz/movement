use crate::EXEC_LOG_FILTER;
use anyhow::Result;

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> ee39c4f3 (first version of the Test runtime. Implements only Load test.)
/// A scenario is any struct that implements the Scenario trait.
/// To ease scenario execution login and using id (usize) is provided during creation.
/// If you use it, it can be return after the scenario execution otherwise return any usize.
/// This id is only use to identify scenario execution is the logs.
<<<<<<< HEAD

/// Implements this trait to develop a scenario.
/// Scenario execution logs:
///  * on std out using tracing::{error, warn, info, debug, trace}
///  * the same logs goes in the log file defined in the config.
///  * to log in the execution json formated file use the log_exec_info function opf the trait.
///
///  Return the execution result (Result<(), TestExecutionError>) and a usize, id of the scenario or any usize.
#[async_trait::async_trait]
pub trait Scenario {
	async fn run(self: Box<Self>) -> Result<usize>;

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
=======
>>>>>>> ee39c4f3 (first version of the Test runtime. Implements only Load test.)

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
<<<<<<< HEAD
>>>>>>> ff595882 (first version of the scenario executor.)
=======

/// Type definition that is used by the test executor to create scenario to execute.
pub type CreateScenarioFn = (dyn Fn(usize) -> Box<dyn Scenario> + Send + Sync);
>>>>>>> ee39c4f3 (first version of the Test runtime. Implements only Load test.)
