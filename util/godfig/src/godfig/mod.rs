use crate::backend::{
    BackendOperations,
    config_file::ConfigFile
};

use std::sync::Arc;
use std::marker::PhantomData;
use serde::de::DeserializeOwned;
use serde::Serialize;

// Define a trait BackendOperations that Backend will implement.
pub trait BackendOperations {
    // Add required methods here
}

#[derive(Debug, Clone)]
pub struct Godfig<Backend, Contract>
where
    Backend: BackendOperations + Send + Sync,
    Contract: DeserializeOwned + Serialize,
{
    backend: Arc<dyn BackendOperations>,
    _marker: PhantomData<Contract>,
    key : Vec<String>,
}

impl<Backend, Contract> Godfig<Backend, Contract>
where
    Backend: BackendOperations + Send + Sync,
    Contract: DeserializeOwned + Serialize,
{

}
