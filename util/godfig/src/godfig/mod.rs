use crate::backend::{
    BackendOperations,
    GodfigBackendError,
};

use std::marker::PhantomData;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Godfig<Backend, Contract>
where
    Backend: BackendOperations,
    Contract: DeserializeOwned + Serialize + Send,
{
    backend: Backend,
    _marker: PhantomData<Contract>,
    key : Vec<String>,
}

impl<Backend, Contract> Godfig<Backend, Contract>
where
    Backend: BackendOperations,
    Contract: DeserializeOwned + Serialize + Send,
{

    pub async fn try_transaction<F, Fut>(&self, callback: F) -> Result<(), GodfigBackendError>
    where
        F: FnOnce(Option<Contract>) -> Fut + Send,
        Fut: std::future::Future<Output = Result<Option<Contract>, GodfigBackendError>> + Send
    {
        let key = self.key.clone();
        let res = self.backend.try_transaction::<Vec<String>, Contract, F, Fut>(key, callback).await;
        res
    }

    pub async fn try_wait_for_ready(&self) -> Result<Contract, GodfigBackendError> {
        let key = self.key.clone();
        self.backend.try_wait_for::<Vec<String>, Contract>(key).await
    }

}
