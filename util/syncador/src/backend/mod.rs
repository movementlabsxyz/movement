// pub mod archive;
// pub mod copy;
pub mod archive;
pub mod clear;
pub mod constant;
pub mod glob;
pub mod pipeline;
pub mod s3;
use crate::files::package::Package;
#[async_trait::async_trait]
pub trait PushOperations {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error>;
}

#[async_trait::async_trait]
pub trait PullOperations {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error>;
}
