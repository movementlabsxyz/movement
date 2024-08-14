// pub mod archive;
// pub mod copy;
pub mod archive;
pub mod pipeline;
pub mod s3;
use crate::files::package::Package;
#[async_trait::async_trait]
pub trait BackendOperations {
	/// Uploads a package to the backend.
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error>;

	/// Downloads a package from the backend.
	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error>;
}

#[async_trait::async_trait]
pub trait PushOperations {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error>;
}

#[async_trait::async_trait]
pub trait PullOperations {
	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error>;
}
