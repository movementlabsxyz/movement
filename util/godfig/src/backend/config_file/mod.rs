use flocks::tfrwlock::TfrwLock;
use std::sync::Arc;
use std::fs::File;

#[derive(Clone)]
pub struct ConfigFile {
    pub (crate) lock: Arc<TfrwLock<File>>,
}

impl ConfigFile {
    
    pub fn new(file: File) -> Self {
        Self {
            lock: Arc::new(TfrwLock::new(file))
        }
    }

    pub async fn do_thing(&self) -> Result<(), anyhow::Error> {
        let _write_guard = self.lock.write().await?;
        Ok(())
    }

}

#[cfg(test)]
pub mod test {
    use super::*;

    #[tokio::test]
    async fn test_locking() -> Result<(), anyhow::Error> {
        let file = File::create("test.txt")?;
        let config_file = ConfigFile::new(file);

        // cannot read and write at the same time
        let write_guard = config_file.lock.write().await?;
        // trying to acquire a read guard should now fail
        let read_result = tokio::time::timeout(std::time::Duration::from_millis(100), config_file.lock.read()).await;

        assert!(read_result.is_err(), "Read lock should not be acquired while holding write lock");

        Ok(())
    }
}