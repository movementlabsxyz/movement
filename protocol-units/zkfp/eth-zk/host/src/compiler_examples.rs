use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

use foundry_compilers::{info::ContractInfo, Artifact, Project, ProjectCompileOutput, ProjectPathsConfig};

/// Makes a foo function that is compatible with the expectations of the guest code.
pub fn compile_foo(structs: &[&str], fun_sig: &str, fun_body: &str) -> Vec<u8> {
    let structs = structs.to_vec().join("\n");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = ProjectPathsConfig::builder()
        .sources(root.join("contracts"))
        .build()
        .unwrap();
    let project = Project::builder()
        .paths(paths)
        .no_artifacts()
        .build()
        .unwrap();

    let compiled = project.compile().unwrap();
    let info = ContractInfo::new("contracts/Foo.sol:Foo");
    let bytes = compiled
        .find_contract(info)
        .expect("Could not find contract")
        .get_bytecode_bytes()
        .unwrap()
        .to_vec();
    bytes
}
