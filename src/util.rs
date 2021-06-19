//! Helper functions and trait

use async_trait::async_trait;

/// .await until the end of the task in a blocking manner
pub trait Conclude {
    /// .await until the end of the task
    fn conclude(&mut self);
}

#[cfg(feature = "async-std")]
impl Conclude for async_std::task::JoinHandle<()> {
    fn conclude(&mut self) {
        async_std::task::block_on(self)
    }
}

#[cfg(feature = "tokio")]
impl Conclude for tokio::task::JoinHandle<()> {
    fn conclude(&mut self) {
        tokio::task::block_in_place(
            || tokio::runtime::Handle::current().block_on(self)
        )
        .unwrap_or_else(|err| log::error!("{:?}", err)) 
    }
}

/// This trait simply cancel/abort the task during execution
#[async_trait]
pub trait Terminate {
    /// Interrupt execution of the task
    async fn terminate(self);
}

#[cfg(feature = "async-std")]
#[async_trait]
impl Terminate for async_std::task::JoinHandle<()> {
    async fn terminate(self) {
        self.cancel().await;
    }
}

#[cfg(feature = "tokio")]
#[async_trait]
impl Terminate for tokio::task::JoinHandle<()> {
    async fn terminate(self) {
        self.abort();
    }
}