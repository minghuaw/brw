//! Broker trait definition
use std::sync::Arc;
use async_trait::async_trait;
use futures_util::{Future, FutureExt, sink::{Sink}, stream::{Stream, StreamExt}, select};
use tokio::sync::{Mutex, mpsc::{error::SendError}, oneshot};

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
        ctx: &Arc<Mutex<Context<Self::Item>>>,
        item: Self::Item,
        writer: W,
    ) -> Running<Result<Self::Ok, Self::Error>, Self::Error>
    where W: Sink<Self::WriterItem, Error = SendError<Self::WriterItem>> + Send + Unpin; // None will stop the loop

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<(), Self::Error> {
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
        ctx: Arc<Mutex<Context<Self::Item>>>,
        mut stop: oneshot::Receiver<()>,
        reader_handle: H, 
        writer_handle: H
    ) -> Result<(), Self::Error>
    where 
        S: Stream<Item = Self::Item> + Send + Unpin,
        W: Sink<Self::WriterItem, Error = SendError<Self::WriterItem>> + Send + Unpin,
        H: Future + Send,
    {
        let this = &mut self;
        let f = Self::handle_result;
        let stop_mut = &mut stop;
        loop {
            select! {
                _ = stop_mut.fuse() => {
                    break;
                },
                item = items.next().fuse() => {
                    if let Some(item) = item {
                        match this.op(&ctx, item, &mut writer).await {
                            Running::Continue(res) => {
                                match f(res).await {
                                    Running::Continue(_) => { },
                                    Running::Stop(res) => {
                                        return res
                                    }
                                }
                            },
                            // None is used to indicate stopping the loop
                            Running::Stop(res) => return res
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

        {
            let mut guard = ctx.lock().await;
            // Stop the reader
            if let Some(reader_stop) = guard.reader_stop.take() {
                if reader_stop.send(()).is_ok() {
                    reader_handle.await;
                }
            }
        }

        #[cfg(feature = "debug")]
        log::debug!("Exiting broker loop");
        return Ok(())
    }
}