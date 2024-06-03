/// A simple demo scenario that sleep a few milli second and log some messages.
/// To run it use: cargo run --release --bin demoscenario
use load_soak_testing::execute_test;
use load_soak_testing::init_test;
use load_soak_testing::ExecutionConfig;
use load_soak_testing::Scenario;
use load_soak_testing::TestExecutionError;

fn main() {
	// Define the Test config. Use the default parameters.
	let config = ExecutionConfig::default();

	// Init the Test before execution
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}

	// Execute the test.
	let result = execute_test(config, &create_demo_scenario);
	tracing::info!("End Test with result {result:?}",);
}

// Scenario constructor function use by the Test runtime to create new scenarios.
fn create_demo_scenario(id: usize) -> Box<dyn Scenario> {
	Box::new(ScenarioDemo { id })
}

pub struct ScenarioDemo {
	id: usize,
}

impl ScenarioDemo {
	pub fn new(id: usize) -> Self {
		ScenarioDemo { id }
	}
}

// Scenario trait implementation.
#[async_trait::async_trait]
impl Scenario for ScenarioDemo {
	async fn run(self: Box<Self>) -> (usize, Result<(), TestExecutionError>) {
		// Trace in the log file and stdout.
		tracing::info!("Scenarios:{} start", self.id);
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		// Trace in the json formated execution log file.
		self.log_exec_info(&format!("Scenario:{} ended", self.id));
		(self.id, Ok(()))
	}
}
