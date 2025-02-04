use clap::Parser;
use clap::Subcommand;
use movement_config::Config;
use syncador::PushOperations;
use syncup::Syncupable;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Backup {
	Db(BackupParam),
	Push(PushParam),
}

impl Backup {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Backup::Db(param) => param.execute().await,
			Backup::Push(param) => param.execute().await,
		}
	}
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Backup the specified Node db at root_dir.")]
pub struct BackupParam {
	#[clap(default_value = "{maptos,maptos-storage,movement-da-db}/**", value_name = "DB PATTERN")]
	pub db_sync: String,
	#[clap(value_name = "DIRECTORY")]
	pub root_dir: Option<String>,
}

impl BackupParam {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let root_path = match &self.root_dir {
			Some(path) => {
				let path = std::path::Path::new(&path);
				if path.exists() {
					path.to_path_buf()
				} else {
					let mut root_path =
						std::env::current_dir().expect("Current working dir not defined.");
					root_path.push(&path);
					root_path
				}
			}
			None => {
				let dot_movement = dot_movement::DotMovement::try_from_env()?;
				dot_movement.get_path().to_path_buf()
			}
		};

		println!("root_path: {:?}", root_path);

		let archive_pipe = syncador::backend::pipeline::push::Pipeline::new(vec![
			Box::new(syncador::backend::glob::file::FileGlob::try_new(
				&self.db_sync.clone(),
				root_path.clone(),
			)?),
			Box::new(syncador::backend::archive::gzip::push::Push::new(root_path)),
		]);

		match archive_pipe.push(syncador::Package::null()).await {
			Ok(package) => {
				tracing::info!("Backup done in file: {:?}", package);
			}
			Err(err) => {
				tracing::warn!("Error during backup: {:?}", err);
			}
		}

		Ok(())
	}
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Backup the specified Node db at root_dir.")]
pub struct PushParam {
	#[clap(default_value = "mtnet-l-sync-bucket-sync", value_name = "BUCKET NAME")]
	pub bucket: String,
}

impl PushParam {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		//Load node config.
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement.try_get_config_from_json::<Config>()?;
		let application_id = config.syncing.try_application_id()?;
		let syncer_id = config.syncing.try_syncer_id()?;
		let s3_push = syncador::backend::s3::shared_bucket::create_push_with_load_from_env(
			self.bucket.clone(),
			syncador::backend::s3::shared_bucket::metadata::Metadata::default()
				.with_application_id(application_id)
				.with_syncer_id(syncer_id),
		)
		.await?;

		let push_pipe = syncador::backend::pipeline::push::Pipeline::new(vec![Box::new(s3_push)]);

		match push_pipe.push(syncador::Package::null()).await {
			Ok(package) => {
				tracing::info!("Archive file pushed {:?}", package);
			}
			Err(err) => {
				tracing::warn!("Error during archive push: {:?}", err);
			}
		}

		Ok(())
	}
}
