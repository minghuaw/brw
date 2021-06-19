//! Builder for broker-reader-writer

#[cfg(any(
    all(feature = "tokio", not(feature = "async-std")),
    all(feature = "async-std", not(feature = "tokio"))
))]
use futures::sink::Sink;

use crate::{broker::Broker, reader::Reader, writer::Writer};

/// Builder for Broker-Reader-Writer pattern
pub struct Builder<B, R, W> {
    broker: Option<B>,
    reader: Option<R>,
    writer: Option<W>,
}

impl<B, R, W> Builder<B, R, W> {
    /// Creates a new but empty builder
    pub fn new() -> Self {
        Builder::<B, R, W> {
            broker: None,
            reader: None,
            writer: None,
        }
    }

    /// Sets the broker
    pub fn set_broker<BB: Broker + Send>(self, broker: BB) -> Builder<BB, R, W> {
        Builder::<BB, R, W> {
            broker: Some(broker),
            reader: self.reader,
            writer: self.writer,
        }
    }

    /// Sets the reader
    pub fn set_reader<RR: Reader + Send>(self, reader: RR) -> Builder<B, RR, W> {
        Builder::<B, RR, W> {
            broker: self.broker,
            reader: Some(reader),
            writer: self.writer,
        }
    }

    /// Sets the writer
    pub fn set_writer<WW: Writer + Send>(self, writer: WW) -> Builder<B, R, WW> {
        Builder::<B, R, WW> {
            broker: self.broker,
            reader: self.reader,
            writer: Some(writer),
        }
    }
}

#[cfg(any(
    all(feature = "tokio", not(feature = "async-std")),
    all(feature = "async-std", not(feature = "tokio"))
))]
impl<B, R, W, BI, WI> Builder<B, R, W> 
where 
    B: Broker<Item = BI, WriterItem = WI> + Send + 'static,
    R: Reader<BrokerItem = BI> + Send + 'static,
    W: Writer<Item = WI> + Send + 'static,
    BI: Send + 'static,
    WI: Send + 'static,
{
    /// Spawning a broker-reader-writer with `tokio` runtime
    #[cfg(all(feature = "tokio", not(feature = "async-std")))]
    pub fn spawn(self) -> Option<(tokio::task::JoinHandle<()>, impl Sink<B::Item>)> {
        let ret = super::spawn(self.broker?, self.reader?, self.writer?);
        Some(ret)
    }

    /// Spawning a broker-reader-writer with `async-std` runtime
    #[cfg(all(feature = "async-std", not(feature = "tokio")))]
    pub fn spawn(self) -> Option<(async_std::task::JoinHandle<()>, impl Sink<B::Item>)> {
        let ret = super::spawn(self.broker?, self.reader?, self.writer?);
        Some(ret)
    }
}