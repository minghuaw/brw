use async_trait::async_trait;
use futures::{
    sink::{Sink},
};

#[async_trait]
pub trait Reader: Sized {
    type BrokerItem: Send + 'static;
    type Ok: Send;
    type Error: std::error::Error;

    async fn op(
        &mut self,
        broker: impl Sink<Self::BrokerItem>
    ) -> Option<Result<Self::Ok, Self::Error>>;

    async fn reader_loop<B>(mut self, mut broker: B)
    where 
        B: Sink<Self::BrokerItem> + Send + Unpin
    {
        loop {
            match self.op(&mut broker).await {
                Some(res) => {
                    if let Err(err) = res {
                        log::error!("{:?}", err);
                    }
                },
                None => break
            }
        }
    }
}