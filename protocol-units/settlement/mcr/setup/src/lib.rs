use dot_movement::DotMovement;
use mcr_settlement_config::Config;

pub mod local;
pub mod deploy;

#[derive(Debug, Clone, Default)]
pub struct Setup {
	pub local: local::Local,
	pub deploy: deploy::Deploy,
}

impl Setup {
	pub fn new() -> Self {
		Self {
			local: local::Local::new(),
			deploy: deploy::Deploy::new(),
		}
	}

	pub async fn setup (
		&self,
		dot_movement: &DotMovement,
		mut config: Config,
	) -> Result<(Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>), anyhow::Error> {
		
		let join_handle = if config.should_run_local() {
			let (new_config, handle) = self.local.setup(dot_movement, config).await?;
			config = new_config;
			handle
		} else {
			tokio::spawn(async { std::future::pending().await })
		};
	
		if let Some(deploy) = &config.deploy {
			config = self.deploy.setup(dot_movement, config.clone(), deploy).await?;
		}
	
		Ok((config, join_handle))

	}

}

