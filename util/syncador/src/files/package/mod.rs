use std::path::{Path, PathBuf};

/// A package is a collection of file system locations that are synced together, either publicly or privately.
#[derive(Clone)]
pub struct Package(pub Vec<PackageElement>);

impl Package {
	/// Returns references to all of the package manifests in the package.
	pub fn as_manifests(&self) -> Vec<&PackageElement> {
		self.0.iter().collect()
	}

	/// Returns ownership of all of the package manifests in the package.
	pub fn into_manifests(self) -> Vec<PackageElement> {
		self.0.into_iter().collect()
	}
}

#[derive(Debug, Clone)]
pub struct PackageElement {
	/// The files that are synced together.
	pub sync_files: Vec<PathBuf>,
	/// The root directory of the package.
	pub root_dir: PathBuf,
}

impl PackageElement {
	/// Creates a new package element with the given root directory.
	pub fn new(root_dir: PathBuf) -> Self {
		Self { sync_files: Vec::new(), root_dir }
	}

	pub fn try_path_tuples(&self) -> Result<Vec<(&Path, &PathBuf)>, anyhow::Error> {
		let mut tuples = Vec::new();
		for file in &self.sync_files {
			let relative_path = file.strip_prefix(&self.root_dir)?;
			tuples.push((relative_path, file));
		}
		Ok(tuples)
	}

	pub fn add_sync_file(&mut self, file: PathBuf) {
		self.sync_files.push(file);
	}

	pub fn sync_files(&self) -> Vec<&PathBuf> {
		self.sync_files.iter().collect()
	}

	pub fn root_dir(&self) -> &PathBuf {
		&self.root_dir
	}
}
