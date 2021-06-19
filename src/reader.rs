//! Reader trait definition

use async_trait::async_trait;
use futures::{
    sink::{Sink},
};

/// Reader of the broker-reader-writer pattern
#[async_trait]
pub trait Reader: Sized {
    /// Item to send to broker
    type BrokerItem: Send + 'static;
    /// Ok result from `op`
    type Ok: Send;
    /// Error result from `op`
    type Error: std::error::Error + Send;

    /// The operation to perform
    async fn op(&mut self, broker: impl Sink<Self::BrokerItem>) -> Option<Result<Self::Ok, Self::Error>>;

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Option<()>;

    /// Runs the operation in a loop
    async fn reader_loop<B>(mut self, mut broker: B)
    where 
        B: Sink<Self::BrokerItem> + Send + Unpin
    {
        loop {
            match self.op(&mut broker).await {
                Some(res) => {
                    if <Self as Reader>::handle_result(res).await.is_none() {
                        break;
                    }
                },
                None => break
            }
        }
    }
}