use clap::Parser;
use clap::Subcommand;
use movement_config::Config;
use std::path::PathBuf;
use syncador::PullOperations;
use syncador::PushOperations;
use syncup::Syncupable;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Backup {
	Save(SaveDbParam),
	Push(PushParam),
	SaveAndPush(SaveAndPush),
	Restore(RestoreParam),
}

impl Backup {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Backup::Save(param) => param.execute().await,
			Backup::Push(param) => param.execute().await,
			Backup::Restore(param) => param.execute().await,
			Backup::SaveAndPush(param) => param.execute().await,
		}
	}
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Save the db using db_sync pattern in root_dir.")]
pub struct SaveDbParam {
	#[clap(default_value = "{maptos,maptos-storage,movement-da-db}/**", value_name = "DB PATTERN")]
	pub db_sync: String,
	#[clap(value_name = "ROOT DIRECTORY")]
	pub root_dir: Option<String>,
}

impl SaveDbParam {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let root_path = get_root_path(self.root_dir.as_ref())?;

		tracing::info!("Save db with parameters: sync:{} root dir:{:?}", self.db_sync, root_path);

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
#[clap(rename_all = "kebab-case", about = "Push the archived db to the bucket")]
pub struct PushParam {
	#[clap(default_value = "follower-test-ci-backup", value_name = "BUCKET NAME")]
	pub bucket: String,
	#[clap(default_value = "0.tar.gz", value_name = "ARCHIVE FILENAME")]
	pub archive_file: String,
	#[clap(value_name = "ROOT DIRECTORY")]
	pub root_dir: Option<String>,
}

impl PushParam {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		//Load node config.
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement.try_get_config_from_json::<Config>()?;
		let application_id = config.syncing.try_application_id()?;
		let syncer_id = config.syncing.try_syncer_id()?;
		let root_path = get_root_path(self.root_dir.as_ref())?;

		tracing::info!(
			"Push db with parameters: bucket:{} archive_file:{} root dir:{:?}",
			self.bucket,
			self.archive_file,
			root_path
		);

		let s3_push = syncador::backend::s3::shared_bucket::create_push_with_load_from_env(
			self.bucket.clone(),
			syncador::backend::s3::shared_bucket::metadata::Metadata::default()
				.with_application_id(application_id)
				.with_syncer_id(syncer_id),
		)
		.await?;

		let push_pipe = syncador::backend::pipeline::push::Pipeline::new(vec![Box::new(s3_push)]);

		let archive_file = root_path.join(&self.archive_file);

		let package = syncador::Package(vec![syncador::PackageElement {
			sync_files: vec![archive_file],
			root_dir: root_path,
		}]);

		match push_pipe.push(package).await {
			Ok(package) => {
				tracing::info!("Push done {:?}", package);
			}
			Err(err) => {
				tracing::warn!("Error during archive push: {:?}", err);
			}
		}

		Ok(())
	}
}

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Save the db using db_sync pattern in root_dir then push it to the bucket."
)]
pub struct SaveAndPush {
	#[clap(default_value = "follower-test-ci-backup", value_name = "BUCKET NAME")]
	pub bucket: String,
	#[clap(default_value = "{maptos,maptos-storage,movement-da-db}/**", value_name = "DB PATTERN")]
	pub db_sync: String,
	#[clap(default_value = "0.tar.gz", value_name = "ARCHIVE FILENAME")]
	pub archive_file: String,
	#[clap(value_name = "ROOT DIRECTORY")]
	pub root_dir: Option<String>,
}

impl SaveAndPush {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let root_path = get_root_path(self.root_dir.as_ref())?;

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

		tracing::info!(
			"Save and Push db with parameters: bucket:{} sync:{} archive_file:{} root dir:{:?}",
			self.bucket,
			self.db_sync,
			self.archive_file,
			root_path
		);

		let push_pipe = syncador::backend::pipeline::push::Pipeline::new(vec![
			Box::new(syncador::backend::glob::file::FileGlob::try_new(
				&self.db_sync.clone(),
				root_path.clone(),
			)?),
			Box::new(syncador::backend::archive::gzip::push::Push::new(root_path)),
			Box::new(s3_push),
		]);

		match push_pipe.push(syncador::Package::null()).await {
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
#[clap(
	rename_all = "kebab-case",
	about = "Restore from the specified bucket in the root_dir. Db pattern is used to clean before the update."
)]
pub struct RestoreParam {
	#[clap(default_value = "follower-test-ci-backup", value_name = "BUCKET NAME")]
	pub bucket: String,
	#[clap(default_value = "{maptos,maptos-storage,movement-da-db}/**", value_name = "DB PATTERN")]
	pub db_sync: String,
	#[clap(value_name = "ROOT DIRECTORY")]
	pub root_dir: Option<String>,
}

impl RestoreParam {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let root_path = get_root_path(self.root_dir.as_ref())?;

		//Load node config.
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement.try_get_or_create_config_from_json::<Config>()?;
		let application_id = config.syncing.try_application_id()?;
		let syncer_id = config.syncing.try_syncer_id()?;

		tracing::info!(
			"Restore db with parameters: bucket:{} sync:{} root dir:{:?}",
			self.bucket,
			self.db_sync,
			root_path
		);

		let s3_pull = syncador::backend::s3::shared_bucket::create_pull_with_load_from_env(
			self.bucket.clone(),
			syncador::backend::s3::shared_bucket::metadata::Metadata::default()
				.with_application_id(application_id)
				.with_syncer_id(syncer_id),
			root_path.clone(),
		)
		.await?;

		let push_pipe = syncador::backend::pipeline::pull::Pipeline::new(vec![
			Box::new(s3_pull),
			Box::new(syncador::backend::clear::glob::pull::ClearGlob::try_new(
				&self.db_sync,
				root_path.clone(),
			)?),
			Box::new(syncador::backend::archive::gzip::pull::Pull::new(root_path.clone())),
		]);

		match push_pipe.pull(Some(syncador::Package::null())).await {
			Ok(package) => {
				tracing::info!("Files restored");
			}
			Err(err) => {
				tracing::warn!("Error during archive push: {:?}", err);
			}
		}

		Ok(())
	}
}

fn get_root_path(initial_dir: Option<&String>) -> Result<PathBuf, anyhow::Error> {
	match initial_dir {
		Some(path) => {
			let path = std::path::Path::new(&path);
			if path.exists() {
				Ok(path.to_path_buf())
			} else {
				let mut root_path =
					std::env::current_dir().expect("Current working dir not defined.");
				root_path.push(&path);
				Ok(root_path)
			}
		}
		None => {
			let dot_movement = dot_movement::DotMovement::try_from_env()?;
			Ok(dot_movement.get_path().to_path_buf())
		}
	}
}