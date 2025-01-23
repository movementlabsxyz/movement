use super::log::{
        KeyRotationMessage, Operation, TransactionId, Wal, WalEntry, StartRotateKey,
        SendAppUpdateKey, RecvAppUpdateKey, SendHsmUpdateKey, RecvHsmUpdateKey,
};
use signing_admin::{
        application::Application,
        backend::SigningBackend,
        key_manager::KeyManager,
};
use anyhow::Result;

pub struct WalExecutor {}

impl WalExecutor {
        pub fn new() -> Self {
                Self {}
        }

        async fn execute_inner<A, B>(
                &self,
                wal: Wal,
                key_manager: &KeyManager<A, B>,
                backend_name: &str, // Added backend_name argument
        ) -> Result<Wal, (Wal, anyhow::Error)>
        where
                A: Application,
                B: SigningBackend,
        {
                let mut wal_clone = wal.clone();

                if let Err(error) = wal_clone.execute(key_manager, backend_name).await {
                        return Err((wal, error));
                }

                Ok(wal_clone)
        }

        pub async fn append<A, B>(
                &self,
                mut wal: Wal,
                key_id: String,
                alias: String,
                key_manager: &KeyManager<A, B>,
                backend_name: &str, // Added backend_name argument
        ) -> Result<Wal, (Wal, anyhow::Error)>
        where
                A: Application,
                B: SigningBackend,
        {
                let transaction_id = TransactionId(uuid::Uuid::new_v4().to_string());

                let wal_entry = WalEntry::new(
                        transaction_id,
                        Operation::RotateKey,
                        vec![
                                KeyRotationMessage::StartRotateKey(StartRotateKey::new(
                                        key_id.clone(),
                                        alias.clone(),
                                )),
                                KeyRotationMessage::SendAppUpdateKey(SendAppUpdateKey::new(
                                        key_id.clone(),
                                        alias.clone(),
                                )),
                                KeyRotationMessage::RecvAppUpdateKey(RecvAppUpdateKey::new(
                                        key_id.clone(),
                                        alias.clone(),
                                )),
                                KeyRotationMessage::SendHsmUpdateKey(SendHsmUpdateKey::new(
                                        key_id.clone(),
                                        alias.clone(),
                                )),
                                KeyRotationMessage::RecvHsmUpdateKey(RecvHsmUpdateKey::new(
                                        key_id.clone(),
                                        alias.clone(),
                                )),
                        ],
                        vec![],
                );

                wal.append(wal_entry);

                // Pass backend_name to execute_inner
                self.execute_inner(wal, key_manager, backend_name).await
        }

        pub async fn execute<A, B>(
                &self,
                wal: Wal,
                key_manager: &KeyManager<A, B>,
                backend_name: &str, // Added backend_name argument
        ) -> Result<Wal, (Wal, anyhow::Error)>
        where
                A: Application,
                B: SigningBackend,
        {
                self.execute_inner(wal, key_manager, backend_name).await
        }

        /// Recovers by replaying and executing any uncommitted entries in the WAL.
        pub async fn recover<A, B>(
                &self,
                wal: Wal,
                key_manager: &KeyManager<A, B>,
                backend_name: &str, // Added backend_name argument
        ) -> Result<Wal, (Wal, anyhow::Error)>
        where
                A: Application,
                B: SigningBackend,
        {
                self.execute_inner(wal, key_manager, backend_name).await
        }
}
