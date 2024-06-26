pub mod frwlock;
pub mod tfrwlock;

pub mod asynchronous {

    use rustix::{
        fd::AsFd,
        fs::{flock as sync_flock, FlockOperation}
    };
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum AsyncFlockError {
        #[error("Lock is not available")]
        JoinError(#[from] tokio::task::JoinError),
        #[error("File error: {0}")]
        IOError(#[from] rustix::io::Errno),
        #[error("Misc: {0}")]
        MiscError(String),
    }
    
    pub async fn flock<Fd: AsFd>(
        file: Fd,
        operation: FlockOperation,
    ) -> Result<(), AsyncFlockError> {
        
        // spawn block and wait for it to finish
        let fd = file.as_fd().try_clone_to_owned().map_err(|e| AsyncFlockError::MiscError(e.to_string()))?;
        tokio::task::spawn_blocking(move || {
            sync_flock(fd, operation)
        }).await??;

        Ok(())

    }

}
