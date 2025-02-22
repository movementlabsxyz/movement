use std::ffi::OsStr;
use std::iter;

#[derive(Debug, Clone)]
pub struct Arabica;

impl Arabica {
	pub fn new() -> Self {
		Arabica
	}

	pub async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: movement_da_util::config::Config,
	) -> Result<(), anyhow::Error> {
		let node_store_dir = dot_movement
			.get_path()
			.join("celestia")
			.join(&config.appd.celestia_chain_id)
			.join(".celestia-light");

		commander::run_command(
			"celestia",
			[
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
				"--node.store",
			]
			.iter()
			.map(AsRef::<OsStr>::as_ref)
			.chain(iter::once(node_store_dir.as_ref())),
		)
		.await?;

		Ok(())
	}
}
