use crate::backend::{
    BackendOperations,
    GodfigBackendError,
};

use std::marker::PhantomData;
use serde::de::DeserializeOwned;
use serde::Serialize;

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

        godfig.try_transaction(|_data| async move {
            Ok(Some(Test {
                test: "test".to_string()
            }))
        }).await?;

        let ready = godfig.try_wait_for_ready().await?;

        assert_eq!(ready.test, "test");

        Ok(())
    }

}