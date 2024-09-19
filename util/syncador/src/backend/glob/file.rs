use std::path::PathBuf;

use crate::backend::{PullOperations, PushOperations};
use crate::files::package::{Package, PackageElement};
use globset::{Glob, GlobSetBuilder};
use tracing::info;

#[derive(Debug, Clone)]
pub struct FileGlob {
	pub pattern: Glob,
	pub root_dir: PathBuf,
}

impl FileGlob {
	pub fn try_new(pattern: &str, root_dir: PathBuf) -> Result<Self, anyhow::Error> {
		// the pattern is actually the pattern applied to the root_dir
		let root_pattern = format!("{}/{}", root_dir.to_string_lossy(), pattern);
		Ok(Self { pattern: Glob::new(root_pattern.as_str())?, root_dir })
	}
}

#[async_trait::async_trait]
impl PullOperations for FileGlob {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error> {
		if package.is_none() {
			return Ok(None);
		}

		let package = package.ok_or(anyhow::anyhow!("package is none"))?;

		Ok(Some(self.push(package).await?))
	}
}

async fn walk_directory(
	root_dir: PathBuf,
	globset: &globset::GlobSet,
) -> Result<Vec<PathBuf>, anyhow::Error> {
	let mut sync_files = Vec::new();
	let mut dirs_to_visit = vec![root_dir];

	while let Some(current_dir) = dirs_to_visit.pop() {
		let mut dir_entries = tokio::fs::read_dir(&current_dir).await?;

		while let Some(dir_entry) = dir_entries.next_entry().await? {
			let path = dir_entry.path();

			if path.is_dir() {
				dirs_to_visit.push(path);
			} else if globset.is_match(path.to_string_lossy().as_ref()) {
				sync_files.push(path);
			}
		}
	}

	Ok(sync_files)
}

#[async_trait::async_trait]
impl PushOperations for FileGlob {
	async fn push(&self, _package: Package) -> Result<Package, anyhow::Error> {
		// just check the matching glob files
		info!("Running glob push");
		let mut globset_builder = GlobSetBuilder::new();
		globset_builder.add(self.pattern.clone());
		let globset = globset_builder.build()?;

		let sync_files = walk_directory(self.root_dir.clone(), &globset).await?;
		info!("Found {} files matching the glob", sync_files.len());

		let package_element = PackageElement { sync_files, root_dir: self.root_dir.clone() };
		Ok(Package(vec![package_element]))
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use crate::backend::PushOperations;
	use crate::files::package::Package;

	#[tokio::test]
	async fn test_file_glob_push() -> Result<(), anyhow::Error> {
		// create a temp dir
		let temp_dir = tempfile::tempdir()?;

		// create some nested files in the temp dir
		let root_dir = temp_dir.path().to_path_buf();
		let nested_dir = root_dir.join("nested");
		std::fs::create_dir_all(&nested_dir)?;
		let file1 = nested_dir.join("file1.txt");
		let file2 = nested_dir.join("file2.txt");
		std::fs::write(&file1, "file1")?;
		std::fs::write(&file2, "file2")?;

		// create another nested dir
		let nested_dir2 = root_dir.join("nested2");
		std::fs::create_dir_all(&nested_dir2)?;
		let file3 = nested_dir2.join("file3.txt");
		std::fs::write(&file3, "file3")?;

		// create a glob for all files nested in "nested" not "nested2"
		let glob = FileGlob::try_new("nested/**/*", root_dir.clone())?;

		// push the glob
		let package = glob.push(Package::null()).await?;

		// check the package
		let Package(package_elements) = package;
		assert_eq!(package_elements.len(), 1);

		// check the first package element
		let PackageElement { sync_files, root_dir } = &package_elements[0];
		assert_eq!(sync_files.len(), 2);
		assert_eq!(root_dir, root_dir);

		// assert the synce files are the expected files
		assert!(sync_files.contains(&file1));
		assert!(sync_files.contains(&file2));

		Ok(())
	}

	#[tokio::test]
	async fn test_bracketed_glob_pattern() -> Result<(), anyhow::Error> {
		// create a temp dir
		let temp_dir = tempfile::tempdir()?;

		// create some nested files in the temp dir
		let root_dir = temp_dir.path().to_path_buf();
		let nested_dir = root_dir.join("nested");
		std::fs::create_dir_all(&nested_dir)?;
		let file1 = nested_dir.join("file1.txt");
		let file2 = nested_dir.join("file2.txt");
		std::fs::write(&file1, "file1")?;
		std::fs::write(&file2, "file2")?;

		// create another nested dir
		let nested_dir2 = root_dir.join("nested2");
		std::fs::create_dir_all(&nested_dir2)?;
		let file3 = nested_dir2.join("file3.txt");
		std::fs::write(&file3, "file3")?;

		// create a third nested dir
		let nested_dir3 = root_dir.join("nested3");
		std::fs::create_dir_all(&nested_dir3)?;
		let file4 = nested_dir3.join("file4.txt");
		std::fs::write(&file4, "file4")?;

		// create a glob for all files nested in "nested", "nested2", and not "nested3"
		let glob = FileGlob::try_new("{nested,nested2}/**/*", root_dir.clone())?;

		// push the glob
		let package = glob.push(Package::null()).await?;

		// check the package
		let Package(package_elements) = package;
		assert_eq!(package_elements.len(), 1);

		// check the first package element
		let PackageElement { sync_files, root_dir } = &package_elements[0];
		assert_eq!(sync_files.len(), 3);
		assert_eq!(root_dir, root_dir);

		// assert the synce files are the expected files
		assert!(sync_files.contains(&file1));
		assert!(sync_files.contains(&file2));
		assert!(sync_files.contains(&file3));

		Ok(())
	}
}
