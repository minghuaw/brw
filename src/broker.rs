//! Broker trait definition

use async_trait::async_trait;
use futures::{
    stream::{Stream, StreamExt},
    sink::{Sink},
};

use super::Running;

use crate::util::{Conclude, Terminate};

/// Broker of the broker-reader-writer pattern
#[async_trait]
pub trait Broker: Sized {
    /// Item sent to the broker
    type Item: Send + 'static;
    /// Item sent to the writer 
    type WriterItem: Send + 'static;
    /// Ok result from `op`
    type Ok: Send;
    /// Error result from `op`
    type Error: std::error::Error + Send;

    /// The operation to perform
    async fn op(
        &mut self, // use self to maintain state
        item: Self::Item,
        writer: impl Sink<Self::WriterItem>
    ) -> Running<Result<Self::Ok, Self::Error>>; // None will stop the loop

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()>;

    /// Runs the operation in a loop
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
                Running::Continue(res) => {
                    match <Self as Broker>::handle_result(res).await {
                        Running::Continue(_) => { },
                        Running::Stop => break
                    }
                },
                // None is used to indicate stopping the loop
                Running::Stop => break
            }
        }

        reader_handle.conclude();
        writer_handle.conclude();
    }
}