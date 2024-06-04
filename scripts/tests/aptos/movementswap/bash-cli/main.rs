
use anyhow::Result;
use load_soak_testing::execute_test;
use load_soak_testing::init_test;
use load_soak_testing::ExecutionConfig;
use load_soak_testing::Scenario;
use 

fn main() {
	println!("Initialize movementswap test...");

	let config = ExecutionConfig::default();
	config.logfile_path = "movementswap.log".to_string();
	config.execfile_path = "./test.sh".to_string();
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}
	let result = execute_test(config);
	tracing::info!("End Test with result {result:?}",);
}

fn create_movementswap_scenario(id: usize) -> Box<dyn Scenario> {
	Box::new(ScenarioSwap { id })
}

pub struct ScenarioSwap {
	id: usize,
}

impl ScenarioSwap {
	pub fn new(id: usize) -> Self {
		ScenarioSwap { id }
	}
}

// Scenario trait implementation.
#[async_trait::async_trait]
impl Scenario for ScenarioSwap {
	async fn run(self: Box<Self>) -> Result<()> {
		// Trace in the log file and stdout.
		tracing::info!("Scenarios:{} start", self.id);
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		// Trace in the json formated execution log file.
		self.log_exec_info(&format!("Scenario:{} ended", self.id));
		Ok(())
	}

	fn get_id(&self) -> usize {
		self.id
	}
}