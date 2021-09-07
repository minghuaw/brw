//! Reader trait definition

use std::sync::Arc;
use async_trait::async_trait;
use futures_util::{
    sink::{Sink},
    FutureExt,
    select
};
use tokio::sync::{Mutex, mpsc::{ error::SendError }, oneshot};

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
    where B: Sink<Self::BrokerItem, Error = SendError<Self::BrokerItem>> + Send + Unpin;

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
    async fn reader_loop<B>(mut self, ctx: Arc<Mutex<Context<Self::BrokerItem>>>, mut broker: B, mut stop: oneshot::Receiver<()>)
    where 
        B: Sink<Self::BrokerItem, Error = SendError<Self::BrokerItem>> + Send + Unpin
    {
        let this = &mut self;
        let f = Self::handle_result;
        let stop_mut = &mut stop;
        loop {
            select! {
                _ = stop_mut.fuse() => {
                    break
                },
                running = this.op(&mut broker).fuse() => {
                    match running {
                        Running::Continue(res) => {
                            match f(res).await {
                                Running::Continue(_) => { },
                                Running::Stop => break
                            }
                        },
                        Running::Stop => break
                    }
                }
            }
        }
        
        {
            let mut guard = ctx.lock().await;
            if let Some(broker_stop) = guard.broker_stop.take() {
                let _ = broker_stop.send(());
            }
        }

        #[cfg(feature = "debug")]
        log::debug!("Exiting reader loop");
    }
}