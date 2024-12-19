#[derive(Debug, Clone)]
pub struct Arabica;

impl Arabica {
	pub fn new() -> Self {
		Arabica
	}

	pub async fn run(
		&self,
		_dot_movement: dot_movement::DotMovement,
		_config: movement_celestia_da_util::config::local::Config,
	) -> Result<(), anyhow::Error> {
		// celestia light start --core.ip validator-1.celestia-arabica-11.com --p2p.network arabica
		commander::run_command(
			"celestia",
			&[
				"light",
				"start",
				"--core.ip",
				"validator-1.celestia-arabica-11.com",
				"--p2p.network",
				"arabica",
				"--log.level",
				"FATAL",
			],
		)
		.await?;

		Ok(())
	}
}
