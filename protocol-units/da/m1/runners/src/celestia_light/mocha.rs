#[derive(Debug, Clone)]
pub struct Mocha;

impl Mocha {
	pub fn new() -> Self {
		Mocha
	}

	pub async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: m1_da_light_node_util::config::local::Config,
	) -> Result<(), anyhow::Error> {
        // celestia light start --core.ip validator-1.celestia-mocha-11.com --p2p.network mocha
        commander::run_command(
            "celestia",
            &[
                "light",
                "start",
                "--core.ip",
                "rpc-mocha.pops.one",
                "--p2p.network",
                "mocha",
                "--log.level",
                "FATAL"
            ],
        ).await?;

		Ok(())
	}
}
