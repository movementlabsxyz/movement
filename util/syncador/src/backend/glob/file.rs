use std::path::PathBuf;

use crate::backend::{PullOperations, PushOperations};
use crate::files::package::{Package, PackageElement};
use glob::{glob, Pattern};

#[derive(Debug, Clone)]
pub struct FileGlob {
	pub pattern: Pattern,
	pub root_dir: PathBuf,
}

impl FileGlob {
	pub fn try_new(pattern: &str, root_dir: PathBuf) -> Result<Self, anyhow::Error> {
		// the pattern is actually the pattern applied to the root_dir
		let root_pattern = format!("{}/{}", root_dir.to_string_lossy(), pattern);

		Ok(Self { pattern: Pattern::new(root_pattern.as_str())?, root_dir })
	}
}

#[async_trait::async_trait]
impl PullOperations for FileGlob {
	async fn pull(&self, _package: Package) -> Result<Package, anyhow::Error> {
		// just check the matching glob files
		let mut sync_files = Vec::new();
		for path in glob(self.pattern.as_str())? {
			sync_files.push(path?);
		}
		let package_element = PackageElement { sync_files, root_dir: self.root_dir.clone() };
		Ok(Package(vec![package_element]))
	}
}

#[async_trait::async_trait]
impl PushOperations for FileGlob {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		self.pull(package).await
	}
}
