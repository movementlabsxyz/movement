use crate::TestExecutionError;
use crate::EXEC_LOG_FILTER;

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
