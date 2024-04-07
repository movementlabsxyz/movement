use tokio::runtime::Handle;
use std::future::Future;
use std::pin::Pin;

// Return a Result<T, E> matching the output of the future.
// T is the type of value the future resolves to, and E is the type of error.
pub fn sync<F, Fut, T, E>(f: F) -> Result<T, E>
where
    F: FnOnce() -> Fut + Send + 'static,  // The closure returns a Future.
    Fut: Future<Output = Result<T, E>> + Send + 'static,  // The Future now returns a Result<T, E>.
    T: Send + 'static,  // The success type.
    E: From<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,  // Error type.
{
    let handle = Handle::current();
    futures::executor::block_on(async move {
        handle
            .spawn(async {
                f().await
            })
            .await
            // Transform panic or spawn error into the function's error type.
            .map_err(|e| E::from(Box::new(e)))?
            // Unwrap the Result<T, E> from the Future.
    })
}

