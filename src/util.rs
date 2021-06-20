//! Helper functions and trait

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
        if let Err(_err) = tokio::task::block_in_place(
            || tokio::runtime::Handle::current().block_on(self)
        ) {
            #[cfg(feature = "log")]
            log::error!("{:?}", _err)
        }
    }
}