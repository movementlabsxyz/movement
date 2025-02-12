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
		// celestia light start --core.ip rpc.celestia.pops.one --p2p.network celestia
		commander::run_command(
			"celestia",
			&[
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
			],
		)
		.await?;

		Ok(())
	}
}
