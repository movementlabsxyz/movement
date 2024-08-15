use crate::backend::PullOperations;
use crate::files::package::{Package, PackageElement};
use flate2::read::GzDecoder;
use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::{fs, task};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate;

#[derive(Debug, Clone)]
pub struct Pull {
	pub destination_dir: PathBuf,
}

impl Pull {
	/// Creates a new Pull instance.
	pub fn new(destination_dir: PathBuf) -> Self {
		Self { destination_dir }
	}

	/// Iteratively collects all files (not directories) in the specified directory using BFS.
	async fn collect_files(dir: &Path, entries: &mut Vec<PathBuf>) -> Result<(), anyhow::Error> {
		let mut queue = VecDeque::new();
		queue.push_back(dir.to_path_buf());

		while let Some(current_dir) = queue.pop_front() {
			let mut read_dir = fs::read_dir(&current_dir).await?;

			while let Some(entry) = read_dir.next_entry().await? {
				let path = entry.path();
				if path.is_dir() {
					queue.push_back(path);
				} else {
					entries.push(path);
				}
			}
		}

		Ok(())
	}

	/// UnGZips and untars a manifest.
	async fn ungzip_tar_manifest(
		manifest: PackageElement,
		destination: PathBuf,
	) -> Result<PackageElement, anyhow::Error> {
		// Create the destination directory if it doesn't exist
		fs::create_dir_all(&destination).await?;

		// Unpack each archive in the manifest
		for (_relative_path, absolute_path) in manifest.try_path_tuples()? {
			let tar_gz = File::open(&absolute_path)?;
			let decoder = GzDecoder::new(tar_gz);
			let mut archive = Archive::new(decoder);

			// Extract the files to the destination directory
			let destination = destination.clone();
			task::spawn_blocking(move || archive.unpack(&destination)).await??;
		}

		// Create a new manifest based on the extracted contents
		let mut new_manifest = PackageElement::new(destination.clone());

		// Recursively add every file (not directory) in the destination directory to the new manifest
		let mut entries = Vec::new();
		Self::collect_files(&destination, &mut entries).await?;
		for file_path in entries {
			new_manifest.add_sync_file(file_path);
		}

		Ok(new_manifest)
	}
}

#[async_trait::async_trait]
impl PullOperations for Pull {
	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error> {
		let mut manifests = Vec::new();
		for manifest in package.0.into_iter() {
			let new_manifest =
				Self::ungzip_tar_manifest(manifest, self.destination_dir.clone()).await?;
			manifests.push(new_manifest);
		}
		Ok(Package(manifests))
	}
}
