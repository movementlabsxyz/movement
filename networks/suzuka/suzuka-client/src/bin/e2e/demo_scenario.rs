use anyhow::Result;
use std::sync::Arc;
/// A simple demo scenario that sleep a few milli second and log some messages.
/// To run it use: cargo run --release --bin demo_scenario
use suzuka_client::load_soak_testing::{
	execute_test, init_test, ExecutionConfig, Scenario, TestKind,
};

fn main() {
	// Define the Test config. Use the default parameters.
	let mut config = ExecutionConfig::default();

	//define soak test. Remove this line for load test.
	config.kind = TestKind::Soak {
		min_scenarios: 6,
		max_scenarios: 10,
		duration: std::time::Duration::from_secs(20),
		number_cycle: 4,
	};

	// Init the Test before execution
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}

	// Execute the test.
	let result = execute_test(config, Arc::new(create_demo_scenario));
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

#[async_trait::async_trait]
impl Scenario for ScenarioDemo {
	async fn run(self: Box<Self>) -> Result<()> {
		// Trace in the log file and stdout.
		tracing::info!("Scenarios:{} start", self.id);
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		// Trace in the json formated execution log file.
		self.log_exec_info(&format!("Scenario:{} ended", self.id));
		Ok(())
	}
}
