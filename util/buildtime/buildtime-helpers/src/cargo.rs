use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::str; // You will need the `serde_json` crate

/// Gets the current cargo workspace root using `cargo metadata`
pub fn cargo_workspace() -> Result<PathBuf, anyhow::Error> {
    let output =
        Command::new("cargo").args(["metadata", "--format-version=1", "--no-deps"]).output()?;

    let metadata = str::from_utf8(&output.stdout)?;
    let json: Value = serde_json::from_str(metadata)?;
    let workspace_root = json["workspace_root"]
        .as_str()
        .ok_or(anyhow::anyhow!("Could not get workspace root from cargo metadata"))?;

    Ok(PathBuf::from(workspace_root))
}

#[cfg(test)]
pub mod test {

    use super::*;
    use std::fs;

    #[test]
    fn test_cargo_workspace() -> Result<(), anyhow::Error> {
        // Get the cargo workspace
        let workspace = cargo_workspace()?;

        // Check that a Cargo.toml file exists in the workspace
        assert_eq!(workspace.join("Cargo.toml").exists(), true);

        // Parse the toml and check that workspace.package.authors is ["Movement Labs"]
        let toml = fs::read_to_string(workspace.join("Cargo.toml"))?;
        let toml: toml::Value = toml::from_str(&toml)?;
        let authors = toml["workspace"]["package"]["authors"].as_array();
        assert_eq!(authors, Some(&vec![toml::Value::String("Movement Labs".to_string())]));

        Ok(())
    }
}
