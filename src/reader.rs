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
    async fn op<B>(&mut self, broker: B) -> Running<Result<Self::Ok, Self::Error>, Option<Self::Error>>
    where B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin;

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<(), Option<Self::Error>> {
        if let Err(_err) = res {
            #[cfg(feature = "debug")]
            log::error!("{:?}", _err);
        }
        Running::Continue(())
    }
    /// Runs the operation in a loop
    async fn reader_loop<B>(mut self, ctx: Arc<Context<Self::BrokerItem>>, mut broker: B, stop: flume::Receiver<()>) -> Result<(), Self::Error>
    where 
        B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin
    {
        let this = &mut self;
        let f = Self::handle_result;
        loop {
            futures::select! {
                _ = stop.recv_async() => {
                    break
                },
                running = this.op(&mut broker).fuse() => {
                    match running {
                        Running::Continue(res) => {
                            match f(res).await {
                                Running::Continue(_) => { },
                                Running::Stop(e) => {
                                    match e {
                                        None => return Ok(()),
                                        Some(err) => return Err(err),
                                    }
                                }
                            }
                        },
                        Running::Stop(e) => {
                            match e {
                                None => return Ok(()),
                                Some(err) => return Err(err),
                            }
                        }
                    }
                }
            }
        }
        if !ctx.broker_stop.is_disconnected() {
            if ctx.broker_stop.send(()).is_ok() { }
        }

        #[cfg(feature = "debug")]
        log::debug!("Exiting reader loop");

        Ok(())
    }
}