use crate::DotMovement;
use syncup::{syncup, Target};

impl DotMovement {
	pub async fn sync(
		&self,
		glob: &str,
		bucket: String,
	) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error> {
		let sync_task = syncup(self.0.clone(), glob, Target::S3(bucket)).await?;
		Ok(sync_task)
	}
}
