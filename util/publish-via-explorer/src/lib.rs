use anyhow::{Context, Result};
use std::{
	env,
	fs::{self, File},
	io::Write,
	path::{Path, PathBuf},
	process::Command,
};

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
	Ok(fs::read(path)?)
}

fn format_vector_u8(data: &[u8]) -> String {
	format!("[{}]", data.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(","))
}

fn format_vector_vector_u8(data: &[Vec<u8>]) -> String {
	serde_json::to_string(data).expect("json encode failed")
}

fn address_from_config() -> Result<String> {
	let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let config_path = crate_dir.join(".movement/config.yaml");
	let contents = fs::read_to_string(&config_path)
		.with_context(|| format!("failed to read {}", config_path.display()))?;

	let config: serde_yaml::Value = serde_yaml::from_str(&contents)?;
	let default = config
		.get("profiles")
		.and_then(|p| p.get("default"))
		.context("missing [profiles][default] in config")?;

	let addr = default
		.get("account")
		.or_else(|| default.get("address"))
		.context("missing 'account' or 'address' under [profiles][default]")?
		.as_str()
		.context("address is not a string")?;

	Ok(addr.to_string())
}

pub fn run() -> Result<()> {
	let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let address = address_from_config()?;

	let move_compile = Command::new("movement")
		.args([
			"move",
			"compile",
			"--named-addresses",
			&format!("hello={}", address),
			"--save-metadata",
			"--package-dir",
			".",
		])
		.current_dir(&crate_dir)
		.status()
		.context("failed to run movement move compile")?;

	if !move_compile.success() {
		anyhow::bail!("Move compilation failed");
	}

	let build_dir = crate_dir.join("build/hello");
	let metadata_path = build_dir.join("package-metadata.bcs");
	let modules_dir = build_dir.join("bytecode_modules");

	let metadata = read_bytes(&metadata_path)?;
	let mut modules = Vec::new();
	for entry in fs::read_dir(&modules_dir)? {
		let entry = entry?;
		if entry.path().extension().map(|ext| ext == "mv").unwrap_or(false) {
			modules.push(read_bytes(&entry.path())?);
		}
	}

	let arg0 = format_vector_u8(&metadata);
	let arg1 = format_vector_vector_u8(&modules);

	let log_path = build_dir.join("explorer_payload.log");
	let mut file = File::create(&log_path)?;
	writeln!(file, "arg0 (vector<u8>):\n{}\n", arg0)?;
	writeln!(file, "arg1 (vector<vector<u8>>):\n{}\n", arg1)?;

	println!("\n----- COPY INTO EXPLORER -----\n");
	println!("arg0 (vector<u8>):\n{}", arg0);
	println!("\narg1 (vector<vector<u8>>):\n{}", arg1);
	println!("\n(Log saved to {})", log_path.display());

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_vector_u8() {
		let input = vec![1, 2, 3, 4, 255];
		let output = format_vector_u8(&input);
		assert_eq!(output, "[1,2,3,4,255]");
	}

	#[test]
	fn test_format_vector_vector_u8() {
		let input = vec![vec![1, 2, 3], vec![4, 5, 6]];
		let output = format_vector_vector_u8(&input);
		assert_eq!(output, "[[1,2,3],[4,5,6]]");
	}

	#[test]
	fn test_read_bytes_failure() {
		let result = read_bytes(Path::new("nonexistent.file"));
		assert!(result.is_err());
	}
}
