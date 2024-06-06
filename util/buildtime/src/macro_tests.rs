#[test]
pub fn test_cargo_workspace() -> Result<(), anyhow::Error> {
    let macro_result = buildtime_macros::cargo_workspace!();
    let runtime_result = buildtime_helpers::cargo::cargo_workspace()?;

    assert_eq!(macro_result, runtime_result);

    Ok(())
}

#[test]
pub fn test_proto() -> Result<(), anyhow::Error> {
    let macro_result = buildtime_macros::proto!();
    let runtime_result = buildtime_helpers::proto::proto()?;

    assert_eq!(macro_result, runtime_result);

    Ok(())
}
