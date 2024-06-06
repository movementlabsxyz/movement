use crate::cargo::cargo_workspace;
use std::path::PathBuf;

pub fn proto() -> Result<PathBuf, anyhow::Error> {
	let workspace = cargo_workspace()?;
	let proto_dir = workspace.join("proto");
	Ok(proto_dir)
}

#[cfg(test)]
pub mod test {

	use super::*;

	#[test]
	fn test_proto() -> Result<(), anyhow::Error> {
		// Get the proto directory
		let proto = proto()?;

		// check that it exists
		assert_eq!(proto.exists(), true);

		Ok(())
	}
}
