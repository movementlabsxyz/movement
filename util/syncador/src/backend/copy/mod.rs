use crate::backend::BackendOperations;
use crate::files::package::{Package, PackageElement};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Copy {
	pub copy_dir: PathBuf,
}

impl Copy {
	pub fn new(copy_dir: PathBuf) -> Self {
		Self { copy_dir }
	}

	pub async fn copy_to_based_on_manifest(
		&self,
		manifest: PackageElement,
	) -> Result<PackageElement, anyhow::Error> {
		let mut path_copy_futures = Vec::new();
		for (relative_path, absolute_path) in manifest.try_path_tuples()? {
			// compute the temp path
			let mut temp_path = self.copy_dir.clone();
			temp_path.push(relative_path);

			path_copy_futures.push(async move {
				// make all of the parent directories
				if let Some(parent) = temp_path.parent() {
					std::fs::create_dir_all(parent)?;
				}

				// copy the file
				std::fs::copy(absolute_path, &temp_path)?;

				Ok::<(PathBuf, PathBuf), anyhow::Error>((relative_path.to_path_buf(), temp_path))
			});
		}

		let put_copy_outputs = futures::future::try_join_all(path_copy_futures).await?;
		let mut new_manifest = PackageElement::empty_matching(&manifest, self.copy_dir.clone());
		for (_, path) in put_copy_outputs {
			new_manifest.add_sync_file(path);
		}

		Ok(new_manifest)
	}
}

#[async_trait::async_trait]
impl BackendOperations for Copy {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		let mut manifest_futures = Vec::new();
		for manifest in package.into_manifests() {
			let future = self.copy_to_based_on_manifest(manifest);
			manifest_futures.push(future);
		}
		let manifests = futures::future::try_join_all(manifest_futures).await?;
		Ok(Package(manifests))
	}

	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error> {
		// same as push
		self.push(package).await
	}
}
