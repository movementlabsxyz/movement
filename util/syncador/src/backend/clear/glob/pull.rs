use crate::backend::glob::file::FileGlob;
use crate::backend::PullOperations;
use crate::files::package::Package;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ClearGlob {
	pub file_glob: FileGlob,
}

impl ClearGlob {
	pub fn try_new(pattern: &str, root_dir: PathBuf) -> Result<Self, anyhow::Error> {
		Ok(Self { file_glob: FileGlob::try_new(pattern, root_dir)? })
	}
}

#[async_trait::async_trait]
impl PullOperations for ClearGlob {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error> {
		// pull the file glob
		let clear_package = self.file_glob.pull(Some(Package::null())).await?;

		if let Some(clear) = clear_package {
			// use tokio fs to remove each file in the package
			for element in clear.into_manifests() {
				for sync_file in element.sync_files {
					let path = element.root_dir.join(sync_file);
					tokio::fs::remove_file(path).await?;
				}
			}
		}

		Ok(package)
	}
}
