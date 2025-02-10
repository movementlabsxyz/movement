use crate::backend::PushOperations;
use crate::files::package::{Package, PackageElement};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::path::PathBuf;
use tar::Builder;

#[derive(Debug, Clone)]
pub struct Push {
	pub archives_dir: PathBuf,
}

impl Push {
	pub fn new(archives_dir: PathBuf) -> Self {
		Self { archives_dir }
	}
}

#[async_trait::async_trait]
impl PushOperations for Push {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		let mut manifests = Vec::new();
		for (i, manifest) in package.0.into_iter().enumerate() {
			let new_manifest = tokio::task::spawn_blocking({
				let archive_dir = self.archives_dir.clone();

				move || {
					tar_gzip_manifest(
						manifest,
						archive_dir.join(format!("{}.tar.gz", i)),
						archive_dir,
					)
				}
			})
			.await??;
			manifests.push(new_manifest);
		}
		Ok(Package(manifests))
	}
}

/// Tar GZips a manifest.
fn tar_gzip_manifest(
	manifest: PackageElement,
	destination: PathBuf,
	root_dir: PathBuf,
) -> Result<PackageElement, anyhow::Error> {
	// create the archive builder
	let file = File::create(destination.clone())?;
	{
		let encoder = GzEncoder::new(file, Compression::default());
		let mut tar_builder = Builder::new(encoder);

		for (relative_path, absolute_path) in manifest.try_path_tuples()? {
			let file = &mut std::fs::File::open(absolute_path)?;
			tar_builder.append_file(relative_path, file)?;
		}

		// Finish writing the tar archive
		tar_builder.finish()?;
	}

	let mut new_manifest = PackageElement::new(root_dir);
	new_manifest.add_sync_file(destination);

	Ok(new_manifest)
}
