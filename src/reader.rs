use async_trait::async_trait;
use futures::{
    sink::{Sink},
};

#[async_trait]
pub trait Reader: Sized {
    type BrokerItem: Send + 'static;
    type Ok: Send;
    type Error: std::error::Error + Send;

    async fn op(&mut self, broker: impl Sink<Self::BrokerItem>) -> Option<Result<Self::Ok, Self::Error>>;

    async fn handle_result(res: Result<Self::Ok, Self::Error>);

    async fn reader_loop<B>(mut self, mut broker: B)
    where 
        B: Sink<Self::BrokerItem> + Send + Unpin
    {
        loop {
            match self.op(&mut broker).await {
                Some(res) => {
                    <Self as Reader>::handle_result(res).await
                },
                None => break
            }
        }
    }
}