use crate::backend::{PullOperations, PushOperations};
use crate::files::package::{Package, PackageElement};
use glob::Pattern;

#[derive(Debug, Clone)]
pub struct PackageGlob {
	pub pattern: Pattern,
}

impl PackageGlob {
	pub fn new(pattern: &str) -> Self {
		Self { pattern: Pattern::new(pattern).unwrap() }
	}

	pub fn is_match(&self, path: &str) -> bool {
		self.pattern.matches(path)
	}
}

#[async_trait::async_trait]
impl PullOperations for PackageGlob {
	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error> {
		let filtered = package
			.0
			.into_iter()
			.map(
				// filter the sync files in the entry that match the glob pattern
				|entry| {
					let sync_files = entry
						.sync_files
						.into_iter()
						.filter(|entry| self.is_match(&entry.to_string_lossy()))
						.collect();
					PackageElement { sync_files, root_dir: entry.root_dir }
				},
			)
			.collect();
		Ok(Package(filtered))
	}
}

#[async_trait::async_trait]
impl PushOperations for PackageGlob {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		self.pull(package).await
	}
}
