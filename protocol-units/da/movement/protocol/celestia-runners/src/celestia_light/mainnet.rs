use commander::Command;

#[derive(Debug, Clone)]
pub struct Mainnet;

impl Mainnet {
	pub fn new() -> Self {
		Mainnet
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
			"rpc.celestia.pops.one",
			"--p2p.network",
			"celestia",
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
