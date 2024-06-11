pub mod local;

#[derive(Debug, Clone)]
pub enum CelestiaBridge {
    Local(local::Local),
}