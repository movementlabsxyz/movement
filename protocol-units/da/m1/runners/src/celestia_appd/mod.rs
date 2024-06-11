pub mod local;

#[derive(Debug, Clone)]
pub enum CelestiaAppd {
    Local(local::Local),
}