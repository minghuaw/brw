use async_trait::async_trait;
use futures::{
    stream::{Stream, StreamExt},
    sink::{Sink},
};

use crate::util::{Conclude, Terminate};

#[async_trait]
pub trait Broker: Sized {
    type Item: Send + 'static;
    type WriterItem: Send + 'static;
    type Ok: Send;
    type Error: std::error::Error + Send;

    async fn op(
        &mut self, // use self to maintain state
        item: Self::Item,
        writer: impl Sink<Self::WriterItem>
    ) -> Option<Result<Self::Ok, Self::Error>>; // None will stop the loop

    async fn handle_result(res: Result<Self::Ok, Self::Error>);

    async fn broker_loop<S, W, H>(
        mut self, 
        mut items: S, 
        mut writer: W, 
        mut reader_handle: H, 
        mut writer_handle: H
    )
    where 
        S: Stream<Item = Self::Item> + Send + Unpin,
        W: Sink<Self::WriterItem> + Send + Unpin,
        H: Conclude + Terminate + Send,
    {
        while let Some(item) = items.next().await {
            match self.op(item, &mut writer).await {
                Some(res) => {
                    <Self as Broker>::handle_result(res).await
                },
                // None is used to indicate stopping the loop
                None => break
            }
        }

        reader_handle.conclude();
        writer_handle.conclude();
    }
}