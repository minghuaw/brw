//! Broker trait definition
use std::sync::Arc;
use async_trait::async_trait;
use flume::Receiver;
use futures::{
    stream::{Stream, StreamExt},
    sink::{Sink},
    FutureExt
};

use super::{Running, Context};

use crate::util::{Conclude};

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
    async fn op<W>(
        &mut self, // use self to maintain state
        ctx: &Arc<Context<Self::Item>>,
        item: Self::Item,
        writer: W,
    ) -> Running<Result<Self::Ok, Self::Error>>
    where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin; // None will stop the loop

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
        if let Err(_err) = res {
            #[cfg(feature = "debug")]
            log::error!("{:?}", _err);
        }
        Running::Continue(())
    }

    /// Runs the operation in a loop
    async fn broker_loop<S, W, H>(
        mut self, 
        mut items: S, 
        mut writer: W, 
        ctx: Arc<Context<Self::Item>>,
        stop: Receiver<()>,
        mut reader_handle: H, 
        mut writer_handle: H
    )
    where 
        S: Stream<Item = Self::Item> + Send + Unpin,
        W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin,
        H: Conclude + Send,
    {
        // let this = &mut self;
        loop {
            futures::select! {
                _ = stop.recv_async() => {
                    break;
                },
                item = items.next().fuse() => {
                    if let Some(item) = item {
                        match self.op(&ctx, item, &mut writer).await {
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
                }
            }
        }

        // Stop the writer
        drop(writer); 
        writer_handle.conclude();

        // Stop the reader
        if !ctx.reader_stop.is_disconnected() {
            if ctx.reader_stop.send(()).is_ok() {
                reader_handle.conclude()
            }
        }

        #[cfg(feature = "debug")]
        log::debug!("Exiting broker loop");
    }
}