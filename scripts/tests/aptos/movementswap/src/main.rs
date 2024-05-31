use load_soak_testing::execute_test;
use load_soak_testing::init_test;
use load_soak_testing::ExecutionConfig;

fn main() {
	println!("Initialize movementswap test...");

	let config = ExecutionConfig::default();
	config.logfile_path = "movementswap.log".to_string();
	config.execfile_path = "../test.sh".to_string();
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}
	let result = execute_test(config);
	tracing::info!("End Test with result {result:?}",);
}