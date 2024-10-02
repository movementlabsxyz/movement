use crate::DotMovement;
use movement_types::application;
use syncup::{syncup, Target};

impl DotMovement {
	pub async fn sync(
		&self,
		is_leader: bool,
		glob: &str,
		bucket: String,
		application_id: application::Id,
	) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error> {
		let sync_task =
			syncup(is_leader, self.0.clone(), glob, Target::S3(bucket), application_id).await?;
		Ok(sync_task)
	}
}
