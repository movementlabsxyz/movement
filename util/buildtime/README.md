# `buildtime`
The `buildtime` crate provides useful functions and macros for use in `build.rs` scripts.

## `workspace`
You can use `cargo_workspace()` or `cargo_workspace!()` to get the path of the workspace root.

```rust
#[test]
pub fn test_cargo_workspace() -> Result<(), anyhow::Error> {

    let macro_result = buildtime_macros::cargo_workspace!();
    let runtime_result = buildtime_helpers::cargo::cargo_workspace()?;

    assert_eq!(macro_result, runtime_result);

    Ok(())

}
```

## `tonic`
Standardizes tonic builds, so that you can simply put:

```rust
buildtime::proto_build_main!("movementlabs/protocol-units/da/light_node/v1beta1.proto");
```

in your `build.rs` file and it will generate the necessary tonic code.

The path provided **MUST** be relative to the `proto` directory in the root of the repository. This is a standard that we have adopted to make it easier to find the proto files.