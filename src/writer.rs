//! Writer trait definition

use async_trait::async_trait;
use futures::{
    stream::{Stream, StreamExt},
};

/// Writer of the broker-reader-writer pattern
#[async_trait]
pub trait Writer: Sized {
    /// Item to receive
    type Item: Send + 'static;
    /// Ok result from `op`
    type Ok: Send;
    /// Ok result from `op`
    type Error: std::error::Error + Send;

    /// The operation to perform
    async fn op(&mut self, item: Self::Item) -> Option<Result<Self::Ok, Self::Error>>;

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Option<()>;

    /// Runs the operation in a loop
    async fn writer_loop<S>(mut self, mut items: S) 
    where 
        S: Stream<Item = Self::Item> + Send + Unpin
    {
        while let Some(item) = items.next().await {
            match self.op(item).await {
                Some(res) => {
                    if <Self as Writer>::handle_result(res).await.is_none() {
                        break;
                    }
                },
                None => break
            }
        }
    }
}