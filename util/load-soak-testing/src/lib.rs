use crate::scenario::CreateScenarioFn;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs::File, sync::Arc};
use tracing_subscriber::{filter, prelude::*};

mod scenario;
pub use scenario::Scenario;

const EXEC_LOG_FILTER: &str = "exec";

/// Initialize all test components with the configuration.
/// Must be call before the test start: execute_test
pub fn init_test(config: &ExecutionConfig) -> Result<(), std::io::Error> {
	//do some verification on the config
	config.verify_config();

	// init tracing to log error in a file and stdout
	// and log execution data in a json file.
	let stdout_log = tracing_subscriber::fmt::layer().pretty();

	// A layer that logs error and warn event to a file.
	let log_file = File::create(&config.logfile_path)?;
	let file_log = tracing_subscriber::fmt::layer().with_writer(Arc::new(log_file));

	// A layer that logs execution event to a file.
	let exec_file = File::create(&config.execfile_path)?;
	let execution_log = tracing_subscriber::fmt::layer().json().with_writer(Arc::new(exec_file));

	tracing_subscriber::registry()
		.with(
			stdout_log
				.with_filter(filter::LevelFilter::INFO)
				.and_then(file_log.with_filter(filter::LevelFilter::WARN))
				// Add a filter that rejects spans and
				// events whose targets start with `exec`.
				.with_filter(filter::filter_fn(|metadata| {
					!metadata.target().starts_with(EXEC_LOG_FILTER)
				})),
		)
		.with(
			// Add a filter to the exec label that *only* enables
			// events whose targets start with `exec`.
			execution_log.with_filter(filter::filter_fn(|metadata| {
				metadata.target().starts_with(EXEC_LOG_FILTER)
			})),
		)
		.init();
	tracing::info!("Load and Soak test inited with config:{config:?}");
	Ok(())
}

/// Define how the test will be run:
/// * kind: Type fo test to run
/// * logfile_path: the file where log WARN and ERROR are written
/// * execfile_path: File where execution data are written to be processed later.
/// * define the number of started scenario per client. nb_scenarios / nb_scenario_per_client define the number of client.
#[derive(Clone, Debug)]
pub struct ExecutionConfig {
	kind: TestKind,
	logfile_path: String,
	execfile_path: String,
	nb_scenario_per_client: usize,
}

impl ExecutionConfig {
	fn verify_config(&self) {
		match self.kind {
			TestKind::Load { nb_scenarios } => {
				assert!(
					nb_scenarios >= self.nb_scenario_per_client,
					"Number of running scenario less than the number if scenario per client."
				);
			},
			TestKind::Soak { min_scenarios, max_scenarios, duration, nb_clycle } => {
				assert!(max_scenarios >= min_scenarios, "max scenarios less than min scenarios");
				assert!(
					min_scenarios >= self.nb_scenario_per_client,
					"Number of min running scenario less than the number if scenario per client."
				);
			},
		}
	}
}

impl Default for ExecutionConfig {
	fn default() -> Self {
		let nb_scenarios: usize = std::env::var("LOADTEST_NB_SCENARIO")
			.unwrap_or("10".to_string())
			.parse()
			.unwrap_or(10);
		let nb_scenario_per_client: usize = std::env::var("LOADTEST_NB_SCENARIO_PER_CLIENT")
			.unwrap_or("2".to_string())
			.parse()
			.unwrap_or(2);
		ExecutionConfig {
			kind: TestKind::build_load_test(nb_scenarios),
			logfile_path: "log_file.txt".to_string(),
			execfile_path: "test_result.txt".to_string(),
			nb_scenario_per_client,
		}
	}
}

/// Define the type of test to Run:
/// * Load: try to run all scenario (nb_scenarios) concurrently
/// * Soak: start min_scenarios at first then increase the number to max_scenarios the decrease and do nb_clycle during duration
#[derive(Clone, Debug)]
pub enum TestKind {
	Load {
		nb_scenarios: usize,
	},
	Soak {
		min_scenarios: usize,
		max_scenarios: usize,
		duration: std::time::Duration,
		nb_clycle: usize,
	},
}

impl TestKind {
	pub fn build_load_test(nb_scenarios: usize) -> Self {
		TestKind::Load { nb_scenarios }
	}
	pub fn build_soak_test(
		min_scenarios: usize,
		max_scenarios: usize,
		duration: std::time::Duration,
		nb_clycle: usize,
	) -> Self {
		TestKind::Soak { min_scenarios, max_scenarios, duration, nb_clycle }
	}
}

/// Execute the test scenarios define in the specified configuration.
/// scenario are executed by chunk. Chunk execution is called client.
/// All clients are executed in a different thread in parallel.
/// Chunk of scenario are executed in a Tokio runtime concurrently.
pub fn execute_test(config: ExecutionConfig, create_scanario: &CreateScenarioFn) {
	tracing::info!("Start test scenario execution.");

	match config.kind {
		TestKind::Load { nb_scenarios } => {
			//build chunk of ids. Start at 1. 0 mean in result execution fail before scenario can execute.
			let ids: Vec<_> = (1..=nb_scenarios).collect();
			let chunks: Vec<_> = ids
				.into_iter()
				.chunks(config.nb_scenario_per_client)
				.into_iter()
				.map(|chunk| chunk.into_iter().collect::<Vec<_>>())
				.collect();
			// Execute the client by id's chunk.
			let exec_results: Vec<_> = chunks
				.par_iter()
				.map(|chunk| {
					let scenarios: Vec<_> =
						chunk.into_iter().map(|id| create_scanario(*id)).collect();
					let client = TestClient::new(scenarios);
					client.run_scenarios()
				})
				.collect();
			let average_exec_time =
				exec_results.iter().map(|res| res.avarage_execution_time_milli).sum::<u128>()
					/ exec_results.len() as u128;
			let metrics_average_exec_time = serde_json::to_string(&average_exec_time)
				.unwrap_or("Metric  execution result serialization error.".to_string());
			tracing::info!(target:EXEC_LOG_FILTER, metrics_average_exec_time);
			tracing::info!("Scenarios execution average_exec_time:{metrics_average_exec_time}");
		},
		TestKind::Soak { min_scenarios, max_scenarios, duration, nb_clycle } => {
			todo!()
		},
	}
	tracing::info!("End test scenario execution.");
}

/// Run the specified scenarios concurrently using Tokio.
#[derive(Default)]
struct TestClient {
	scenarios: Vec<Box<dyn Scenario>>,
}

impl TestClient {
	fn new(scenarios: Vec<Box<dyn Scenario>>) -> Self {
		TestClient { scenarios }
	}

	fn run_scenarios(self) -> ClientExecResult {
		// Start the TOkio runtime on the current thread
		let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
		let scenario_results = rt.block_on(self.runner());

		let exec_results = ClientExecResult::new(scenario_results);
		let metrics_client_execution = serde_json::to_string(&exec_results)
			.unwrap_or("Metric client result serialization error.".to_string());
		tracing::info!(target:EXEC_LOG_FILTER, metrics_client_execution);
		exec_results
	}

	async fn runner(self) -> Vec<ScenarioExecMetric> {
		//start all client's scenario
		let mut set = tokio::task::JoinSet::new();
		let start_time = std::time::Instant::now();
		self.scenarios.into_iter().for_each(|scenario| {
			set.spawn(futures::future::join(
				futures::future::ready(scenario.get_id()),
				scenario.run(),
			));
		});
		let mut scenario_results = vec![];
		while let Some(res) = set.join_next().await {
			let elapse = start_time.elapsed().as_millis();
			let metrics = match res {
				Ok((id, Ok(()))) => ScenarioExecMetric::new_ok(id, elapse),
				Ok((id, Err(err))) => {
					let log = format!("Scenario:{id} execution failed because: {err}");
					tracing::info!(target:EXEC_LOG_FILTER, log);
					tracing::warn!(log);
					ScenarioExecMetric::new_err(id, elapse)
				},
				Err(err) => {
					tracing::warn!("Error during scenario spawning: {err}");
					ScenarioExecMetric::new_err(0, elapse)
				},
			};
			let metrics_scenario = serde_json::to_string(&metrics)
				.unwrap_or("Metric serialization error.".to_string());
			tracing::info!(target:EXEC_LOG_FILTER, metrics_scenario);
			scenario_results.push(metrics);
		}
		scenario_results
	}
}

#[derive(Serialize, Deserialize)]
struct ScenarioExecMetric {
	scenario_id: usize,
	elaspse_millli: u128,
	result: ScenarioExecResult,
}

impl ScenarioExecMetric {
	fn new_ok(scenario_id: usize, elaspse_millli: u128) -> Self {
		ScenarioExecMetric { scenario_id, elaspse_millli, result: ScenarioExecResult::Ok }
	}
	fn new_err(scenario_id: usize, elaspse_millli: u128) -> Self {
		ScenarioExecMetric { scenario_id, elaspse_millli, result: ScenarioExecResult::Fail }
	}

	fn is_ok(&self) -> bool {
		match self.result {
			ScenarioExecResult::Ok => true,
			ScenarioExecResult::Fail => false,
		}
	}
}

#[derive(Serialize, Deserialize)]
enum ScenarioExecResult {
	Ok,
	Fail,
}

#[derive(Serialize, Deserialize)]
struct ClientExecResult {
	avarage_execution_time_milli: u128,
}

impl ClientExecResult {
	fn new(sceanarios: Vec<ScenarioExecMetric>) -> Self {
		ClientExecResult {
			avarage_execution_time_milli: Self::calcualte_average_exec_time_milli(&sceanarios),
		}
	}

	pub fn calcualte_average_exec_time_milli(sceanarios: &[ScenarioExecMetric]) -> u128 {
		if !sceanarios.is_empty() {
			sceanarios
				.iter()
				.filter_map(|s| if s.is_ok() { Some(s.elaspse_millli) } else { None })
				.sum::<u128>() / sceanarios.len() as u128
		} else {
			tracing::warn!("No result available average exec time is 0");
			0
		}
	}
}
