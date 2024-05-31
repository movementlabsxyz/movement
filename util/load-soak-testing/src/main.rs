use load_soak_testing::execute_test;
use load_soak_testing::init_test;
use load_soak_testing::ExecutionConfig;

fn main() {
	println!("Hello, world!");

	let config = ExecutionConfig::default();
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}
	let result = execute_test(config);
	tracing::info!("End Test with result {result:?}",);
}
