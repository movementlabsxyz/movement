use crate::backend::{
    BackendOperations,
    GodfigBackendError,
};

use std::marker::PhantomData;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;

/// A Godfig instance that can be used to interact with a backend.
/// `Contract` is the type of the data that will be stored in the backend. It is a "type contract" that the backend will enforce.
/// `Backend` is the backend that will be used to store the data.
#[derive(Debug, Clone)]
pub struct Godfig<Contract, Backend>
where
    Backend: BackendOperations,
    Contract: DeserializeOwned + Serialize + Send,
{
    backend: Backend,
    _marker: PhantomData<Contract>,
    key : Vec<String>,
}

impl<Contract, Backend> Godfig<Contract, Backend>
where
    Backend: BackendOperations,
    Contract: DeserializeOwned + Serialize + Send,
{

    pub fn new(backend: Backend, key: Vec<String>) -> Self {
        Self {
            backend,
            _marker: PhantomData,
            key,
        }
    }

    /// This function is used to perform a transaction on the backend. The callback function is called with the current value of the key.
    /// The callback function should return a future that resolves to the new value of the key.
    /// `F` is the callback function that will be called with the current value of the key.
    /// `Fut` is the future that the callback function should return.
    pub async fn try_transaction<F, Fut>(&self, callback: F) -> Result<(), GodfigBackendError>
    where
        F: FnOnce(Option<Contract>) -> Fut + Send,
        Fut: Future<Output = Result<Option<Contract>, GodfigBackendError>> + Send
    {
        let key = self.key.clone();
        let res = self.backend.try_transaction::<Vec<String>, Contract, F, Fut>(key, callback).await;
        res
    }

    /// This function is used to perform a transaction on the backend. The callback function is called with the current value of the key.
    /// The callback function should return a future that resolves to a tuple of the new value and the result of the transaction.
    /// `R` is the result of the transaction.
    /// `F` is the callback function that will be called with the current value of the key.
    /// `Fut` is the future that the callback function should return.
    pub async fn try_transaction_with_result<R, F, Fut>(&self, callback: F) -> Result<R, GodfigBackendError>
    where
        F: FnOnce(Option<Contract>) -> Fut + Send,
        Fut: Future<Output = Result<(Option<Contract>, R), GodfigBackendError>> + Send
    {
        let key = self.key.clone();
        let res = self.backend.try_transaction_with_result::<Vec<String>, Contract, R, F, Fut>(key, callback).await;
        res
    }

    pub async fn try_wait_for_ready(&self) -> Result<Contract, GodfigBackendError> {
        let key = self.key.clone();
        self.backend.try_wait_for::<Vec<String>, Contract>(key).await
    }

}


#[cfg(test)]
pub mod test {

    use super::*;
    use crate::backend::config_file::ConfigFile;
    
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Test {
        pub test: String,
    }

    #[tokio::test]
    async fn test_godfig() -> Result<(), GodfigBackendError> {
        let tempfile = tempfile::tempfile()?;
        let backend = ConfigFile::new(tempfile.into());
        let godfig : Godfig<Test, ConfigFile> = Godfig::new(backend, vec!["test".to_string()]);

        godfig.try_transaction(|data| async move {
            assert!(data.is_none());
            Ok(Some(Test {
                test: "test".to_string()
            }))
        }).await?;

        let ready = godfig.try_wait_for_ready().await?;

        assert_eq!(ready.test, "test");

        Ok(())
    }

}