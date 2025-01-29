use anyhow::Context;
use aptos_framework::{BuildOptions, BuiltPackage, ReleaseBundle};
use move_package::source_package::layout::SourcePackageLayout;
use movement::common::utils::write_to_file;
use movement::move_tool::manifest::{
	Dependency, ManifestNamedAddress, MovePackageManifest, PackageInfo,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub struct Compiler {
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: u32,
	pub framework_local_dir: Option<PathBuf>,
}

impl Compiler {
	pub fn new(
		repo: &'static str,
		commit_hash: &'static str,
		bytecode_version: u32,
		framework_local_dir: Option<PathBuf>,
	) -> Self {
		Self { repo, commit_hash, bytecode_version, framework_local_dir }
	}

	/// Initializes a Move package directory with a Move.toml file for the temporary compilation.
	fn init_move_dir(
		&self,
		package_dir: &Path,
		name: &str,
		addresses: BTreeMap<String, ManifestNamedAddress>,
	) -> Result<(), anyhow::Error> {
		const APTOS_FRAMEWORK: &str = "AptosFramework";
		const APTOS_GIT_PATH: &str = "https://github.com/movementlabsxyz/aptos-core.git";
		const SUBDIR_PATH: &str = "aptos-move/framework/aptos-framework";

		let move_toml = package_dir.join(SourcePackageLayout::Manifest.path());

		// Add the framework dependency if it's provided
		let mut dependencies = BTreeMap::new();
		if let Some(ref path) = self.framework_local_dir {
			dependencies.insert(
				APTOS_FRAMEWORK.to_string(),
				Dependency {
					local: Some(path.display().to_string()),
					git: None,
					rev: None,
					subdir: None,
					aptos: None,
					address: None,
				},
			);
		} else {
			dependencies.insert(
				APTOS_FRAMEWORK.to_string(),
				Dependency {
					local: None,
					git: Some(APTOS_GIT_PATH.to_string()),
					rev: Some(self.commit_hash.to_string()),
					subdir: Some(SUBDIR_PATH.to_string()),
					aptos: None,
					address: None,
				},
			);
		}

		let manifest = MovePackageManifest {
			package: PackageInfo {
				name: name.to_string(),
				version: "1.0.0".to_string(),
				license: None,
				authors: vec![],
			},
			addresses,
			dependencies,
			dev_addresses: Default::default(),
			dev_dependencies: Default::default(),
		};

		write_to_file(
			move_toml.as_path(),
			SourcePackageLayout::Manifest.location_str(),
			toml::to_string_pretty(&manifest)
				.map_err(|err| {
					anyhow::anyhow!("failed to serialize the Move package manifest: {}", err)
				})?
				.as_bytes(),
		)
		.map_err(|err| {
			anyhow::anyhow!(
				"failed to write the Move package manifest to {}: {}",
				move_toml.display(),
				err
			)
		})
	}

	/// Compiles a script in a temp dir to bytecode.
	pub fn compile_in_temp_dir_to_bytecode(
		&self,
		script_name: &str,
		script_path: &Path,
	) -> Result<Vec<u8>, anyhow::Error> {
		// build the package
		let built_package = self.compile_in_temp_dir(script_name, script_path)?;

		// get the bytecode; it should located at package_path()/build/script_name/bytecode_scripts/main.mv
		let bytecode_path = built_package
			.package_path()
			.join("build")
			.join(script_name)
			.join("bytecode_scripts")
			.join("main.mv");
		let bytecode = std::fs::read(bytecode_path).context("Failed to read the bytecode file")?;

		Ok(bytecode)
	}

	/// Compiles a script in a temporary directory.
	pub fn compile_in_temp_dir(
		&self,
		script_name: &str,
		script_path: &Path,
	) -> Result<BuiltPackage, anyhow::Error> {
		// Make a temporary directory for compilation
		let package_dir = PathBuf::from(".debug/move-scripts/").join(script_name);

		// Make the temporary directory
		fs::create_dir_all(&package_dir).context(format!(
			"Failed to create the temporary directory {}",
			package_dir.display()
		))?;

		// Initialize the Move package directory
		self.init_move_dir(package_dir.as_path(), script_name, BTreeMap::new())?;

		// Insert the new script
		let sources_dir = package_dir.join("sources");
		let new_script_path = if let Some(file_name) = script_path.file_name() {
			sources_dir.join(file_name)
		} else {
			// If for some reason we can't get the move file
			sources_dir.join("script.move")
		};

		// create parent directories if they don't exist
		fs::create_dir_all(new_script_path.parent().unwrap()).context(format!(
			"Failed to create the parent directories for {}",
			new_script_path.display()
		))?;

		fs::copy(script_path, new_script_path.as_path()).context(format!(
			"Failed to copy the script file {} to the temporary directory",
			script_path.display()
		))?;

		// Compile the script
		self.compile_script(package_dir.as_path(), Some(self.bytecode_version))
	}

	/// Compiles a script in a given directory.
	fn compile_script(
		&self,
		package_dir: &Path,
		bytecode_version: Option<u32>,
	) -> Result<BuiltPackage, anyhow::Error> {
		let build_options = BuildOptions {
			with_srcs: false,
			with_abis: false,
			with_source_maps: false,
			with_error_map: false,
			skip_fetch_latest_git_deps: false,
			bytecode_version,
			..BuildOptions::default()
		};

		let pack = BuiltPackage::build(package_dir.to_path_buf(), build_options)
			.context(format!("Failed to compile the script in {}", package_dir.display()))?;

		Ok(pack)
	}
}
