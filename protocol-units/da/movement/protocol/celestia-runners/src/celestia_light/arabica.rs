use commander::Run;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Arabica;

impl Arabica {
	pub fn new() -> Self {
		Arabica
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
			"validator-1.celestia-arabica-11.com",
			"--p2p.network",
			"arabica",
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
