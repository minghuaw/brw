//! Broker trait definition
use std::sync::Arc;
use async_trait::async_trait;
use flume::Receiver;
use futures::{Future, FutureExt, sink::{Sink}, stream::{Stream, StreamExt}};

use super::{Running, Context};

// use crate::util::{Conclude};

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
        reader_handle: H, 
        writer_handle: H
    )
    where 
        S: Stream<Item = Self::Item> + Send + Unpin,
        W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin,
        H: Future + Send,
    {
        let this = &mut self;
        let f = Self::handle_result;
        loop {
            futures::select! {
                _ = stop.recv_async() => {
                    break;
                },
                item = items.next().fuse() => {
                    if let Some(item) = item {
                        match this.op(&ctx, item, &mut writer).await {
                            Running::Continue(res) => {
                                match f(res).await {
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
        #[cfg(feature = "debug")]
        log::debug!("Dropping writer");
        drop(writer); 
        #[cfg(feature = "debug")]
        log::debug!(".await writer handle");
        let _ = writer_handle.await;

        // Stop the reader
        if !ctx.reader_stop.is_disconnected() {
            if ctx.reader_stop.send_async(()).await.is_ok() {
                let _ = reader_handle.await;
            }
        }

        #[cfg(feature = "debug")]
        log::debug!("Exiting broker loop");
    }
}