use anyhow::Context;
use aptos_framework::{BuildOptions, BuiltPackage, ReleaseBundle, ReleasePackage};
use aptos_gas_schedule::{AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule};
use aptos_release_builder::components::gas::generate_gas_upgrade_proposal;
use aptos_sdk::move_types::gas_algebra::GasQuantity;
use maptos_framework_release_util::{Release, ReleaseBundleError};
use move_package::source_package::layout::SourcePackageLayout;
use movement::common::utils::write_to_file;
use movement::move_tool::manifest::{
	Dependency, ManifestNamedAddress, MovePackageManifest, PackageInfo,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub struct BumpGas {
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: u32,
	pub framework_local_dir: Option<PathBuf>,
}

impl BumpGas {
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

	/// Compiles a script in a temporary directory.
	fn compile_in_temp_dir(
		&self,
		script_name: &str,
		script_path: &Path,
		bytecode_version: Option<u32>,
	) -> Result<BuiltPackage, anyhow::Error> {
		// Make a temporary directory for compilation
		let temp_dir = "debug-temp-dir";

		// Initialize a move directory
		let package_dir = PathBuf::from(temp_dir);
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
		self.compile_script(package_dir.as_path(), bytecode_version)
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

	/// Generates the bytecode for the gas upgrade proposal.
	pub fn bump_gas_proposal_release_pacakge(&self) -> Result<ReleasePackage, ReleaseBundleError> {
		// generate the script
		let mut gas_parameters = AptosGasParameters::initial();
		gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);

		let gas_schedule = aptos_types::on_chain_config::GasScheduleV2 {
			feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
			entries: gas_parameters
				.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
		};

		let (_, update_gas_script) =
			generate_gas_upgrade_proposal(None, &gas_schedule, true, "".to_owned().into_bytes())
				.unwrap()
				.pop()
				.unwrap();

		let temp_dir = tempdir().map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let gas_script_path = temp_dir.path().join("gas_upgrade");
		let mut gas_script_path = gas_script_path.as_path().to_path_buf();
		gas_script_path.set_extension("move");
		fs::write(gas_script_path.as_path(), update_gas_script)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		// list all files in the temp dir
		let files =
			fs::read_dir(temp_dir.path()).map_err(|e| ReleaseBundleError::Build(e.into()))?;
		for file in files {
			let file = file.map_err(|e| ReleaseBundleError::Build(e.into()))?;
			println!("file: {:?}", file.path());
		}

		let package = self
			.compile_in_temp_dir(
				"gas_upgrade",
				gas_script_path.as_path(),
				Some(self.bytecode_version),
			)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let release =
			ReleasePackage::new(package).map_err(|e| ReleaseBundleError::Build(e.into()))?;

		Ok(release)
	}
}

impl Release for BumpGas {
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		let release_packages = vec![self.bump_gas_proposal_release_pacakge()?];
		let release_bundle = ReleaseBundle::new(release_packages, vec![]);
		Ok(release_bundle)
	}
}
