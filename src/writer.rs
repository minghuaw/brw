use async_trait::async_trait;
use futures::{
    stream::{Stream, StreamExt},
};

#[async_trait]
pub trait Writer: Sized {
    type Item: Send + 'static;
    type Ok: Send;
    type Error: std::error::Error + Send;

    async fn op(&mut self, item: Self::Item) -> Option<Result<Self::Ok, Self::Error>>;

    async fn handle_result(res: Result<Self::Ok, Self::Error>);

    async fn writer_loop<S>(mut self, mut items: S) 
    where 
        S: Stream<Item = Self::Item> + Send + Unpin
    {
        while let Some(item) = items.next().await {
            match self.op(item).await {
                Some(res) => {
                    <Self as Writer>::handle_result(res).await
                },
                None => break
            }
        }
    }
}