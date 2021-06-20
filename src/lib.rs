#![warn(missing_docs)]

//! # A builder for the broker-reader-writer pattern

#[cfg(any(
    all(feature = "tokio", not(feature = "async-std")),
    all(feature = "async-std", not(feature = "tokio"))
))]
use futures::sink::Sink;

pub mod util;
pub mod broker;
pub mod reader;
pub mod writer;

#[cfg(feature = "builder")]
pub mod builder;

pub use broker::Broker;
pub use reader::Reader;
pub use writer::Writer;

#[cfg(feature = "builder")]
pub use builder::Builder;

/// Tells whether the loop should continue
pub enum Running<T> {
    /// Continue running
    Continue(T),
    /// Stop running
    Stop,
}

impl<T> From<Option<T>> for Running<T> {
    fn from(val: Option<T>) -> Self {
        match val {
            Some(inner) => Self::Continue(inner),
            None => Self::Stop
        }
    }
}

impl<T> From<Running<T>> for Option<T> {
    fn from(val: Running<T>) -> Self {
        match val {
            Running::Continue(inner) => Some(inner),
            Running::Stop => None
        }
    }
}

/// Spawning a broker-reader-writer with `tokio` runtime
#[cfg(all(feature = "tokio", not(feature = "async-std")))]
pub fn spawn<B, R, W, BI, WI>(broker: B, reader: R, writer: W
) -> (tokio::task::JoinHandle<()>, impl Sink<<B as Broker>::Item>) 
where 
    B: Broker<Item = BI, WriterItem = WI> + Send + 'static,
    R: Reader<BrokerItem = BI> + Send + 'static,
    W: Writer<Item = WI> + Send + 'static,
    BI: Send + 'static,
    WI: Send + 'static,
{
    let (broker_tx, broker_rx) = flume::unbounded();
    let (writer_tx, writer_rx) = flume::unbounded();

    let broker_sink = broker_tx.clone().into_sink();
    let reader_handle = tokio::task::spawn(
        reader.reader_loop(broker_sink)
    );

    let writer_stream = writer_rx.into_stream();
    let writer_handle = tokio::task::spawn(
        writer.writer_loop(writer_stream)  
    );

    let items_stream = broker_rx.into_stream();
    let writer_sink = writer_tx.into_sink();
    let broker_handle = tokio::task::spawn(
        broker.broker_loop(
            items_stream, 
            writer_sink,
            reader_handle,
            writer_handle
        )
    );

    (broker_handle, broker_tx.into_sink())
}

/// Spawning a broker-reader-writer with `async-std` runtime
#[cfg(all(feature = "async-std", not(feature = "tokio")))]
pub fn spawn<B, R, W, BI, WI>(broker: B, reader: R, writer: W
) -> (async_std::task::JoinHandle<()>, impl Sink<<B as Broker>::Item>) 
where 
    B: Broker<Item = BI, WriterItem = WI> + Send + 'static,
    R: Reader<BrokerItem = BI> + Send + 'static,
    W: Writer<Item = WI> + Send + 'static,
    BI: Send + 'static,
    WI: Send + 'static,
{
    let (broker_tx, broker_rx) = flume::unbounded();
    let (writer_tx, writer_rx) = flume::unbounded();

    let broker_sink = broker_tx.clone().into_sink();
    let reader_handle = async_std::task::spawn(
        reader.reader_loop(broker_sink)
    );

    let writer_stream = writer_rx.into_stream();
    let writer_handle = async_std::task::spawn(
        writer.writer_loop(writer_stream)  
    );

    let items_stream = broker_rx.into_stream();
    let writer_sink = writer_tx.into_sink();
    let broker_handle = async_std::task::spawn(
        broker.broker_loop(
            items_stream, 
            writer_sink,
            reader_handle,
            writer_handle
        )
    );

    (broker_handle, broker_tx.into_sink())
}