use commander::Run;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Mocha;

impl Mocha {
	pub fn new() -> Self {
		Mocha
	}

	pub async fn run(
		&self,
		_dot_movement: dot_movement::DotMovement,
		config: movement_da_util::config::Config,
	) -> Result<(), anyhow::Error> {
		let mut command = Command::new("celestia");
		command.args([
			"light",
			"start",
			"--keyring.backend",
			"test",
			"--keyring.keyname",
			&config.light.key_name,
			"--core.ip",
			"rpc-mocha.pops.one",
			"--p2p.network",
			"mocha",
			"--log.level",
			"FATAL",
		]);
		if let Some(path) = &config.light.node_store {
			command.arg("--node.store");
			command.arg(path);
		}
		// FIXME: don't need to capture output
		command.run_and_capture_output().await?;

		Ok(())
	}
}
