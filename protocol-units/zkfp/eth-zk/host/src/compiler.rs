use std::path::Path;

use anyhow::Result;

pub fn compile_units(s: &str) -> Result<Vec<()>> {
    Ok(vec![])
}

fn expect_contracts(units: impl IntoIterator<Item = ()>) -> impl Iterator<Item = Result<()>> {
    units.into_iter().map(|_| Ok(()))
}

pub fn compile_contracts_in_file(_path: &Path) -> Result<Vec<()>> {
    Ok(vec![])
}

#[allow(dead_code)]
pub fn as_contract(_unit: ()) -> () {
    ()
}