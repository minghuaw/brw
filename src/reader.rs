//! Reader trait definition

use std::sync::Arc;
use async_trait::async_trait;
use futures::{
    sink::{Sink},
    FutureExt
};

use super::{Running, Context};

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
    async fn op<B>(&mut self, broker: B) -> Running<Result<Self::Ok, Self::Error>>
    where B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin;

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
        if let Err(err) = res {
            log::error!("{:?}", err);
        }
        Running::Continue(())
    }
    /// Runs the operation in a loop
    async fn reader_loop<B>(mut self, ctx: Arc<Context<Self::BrokerItem>>, mut broker: B, stop: flume::Receiver<()>)
    where 
        B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin
    {
        log::debug!("Reader loop started");
        let this = &mut self;
        loop {
            futures::select! {
                _ = stop.recv_async() => {
                    break
                },
                running = this.op(&mut broker).fuse() => {
                    match running {
                        Running::Continue(res) => {
                            match <Self as Reader>::handle_result(res).await {
                                Running::Continue(_) => { },
                                Running::Stop => break
                            }
                        },
                        Running::Stop => break
                    }
                }
            }
        }

        ctx.broker_stop.send(());
        println!("Dropping reader_loop");
    }
}