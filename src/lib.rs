#![warn(missing_docs)]

//! # A builder for the broker-reader-writer pattern

#[cfg(any(
    all(feature = "tokio", not(feature = "async-std")),
    all(feature = "async-std", not(feature = "tokio"))
))]
use std::sync::Arc;

#[cfg(all(feature = "tokio", not(feature = "async-std")))]
use tokio::sync::{oneshot, mpsc::{self, Sender}};
#[cfg(all(feature = "tokio", not(feature = "async-std")))]
use tokio_util::sync::PollSender;
#[cfg(all(feature = "tokio", not(feature = "async-std")))]
use tokio_stream::wrappers::ReceiverStream;

#[cfg(all(feature = "async-std", not(feature = "tokio")))]
use async_std::channel::{bounded, Sender};

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

impl<T, E> From<Result<T, E>> for Running<Result<T, E>> {
    fn from(res: Result<T, E>) -> Self {
        Running::Continue(res)
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
/// Context of broker-reader-writer
pub struct Context<BI> {
    /// Sender to broker
    pub broker: Sender<BI>, // 
    broker_stop: Option<oneshot::Sender<()>>,
    reader_stop: Option<oneshot::Sender<()>>, // this is the reader stopper
    // writer_stop: Sender<()>, // this is the writer stopper
}

/// Spawning a broker-reader-writer with `tokio` runtime
#[cfg(any(
    feature = "docs",
    all(feature = "tokio", not(feature = "async-std"))
))]
pub fn spawn<B, R, W, BI, WI>(broker: B, reader: R, writer: W, bound: usize
) -> (tokio::task::JoinHandle<()>, Sender<BI>) 
where 
    B: Broker<Item = BI, WriterItem = WI> + Send + 'static,
    R: Reader<BrokerItem = BI> + Send + 'static,
    W: Writer<Item = WI> + Send + 'static,
    BI: Send + 'static,
    WI: Send + 'static,
{
    use tokio::sync::Mutex;

    let (broker_tx, broker_rx) = mpsc::channel(bound);
    let (writer_tx, writer_rx) = mpsc::channel(bound);
    let (reader_stop, stop_reader) = oneshot::channel();
    let (broker_stop, stop_broker) = oneshot::channel();
    let ctx = Context {
        broker: broker_tx.clone(),
        broker_stop: Some(broker_stop),
        reader_stop: Some(reader_stop),
    };
    let ctx = Arc::new(Mutex::new(ctx));

    let broker_sink = PollSender::new(broker_tx.clone());
    let reader_handle = tokio::task::spawn(
        reader.reader_loop(Arc::clone(&ctx), broker_sink, stop_reader)
    );

    let writer_stream = ReceiverStream:: new(writer_rx);
    let writer_handle = tokio::task::spawn(
        writer.writer_loop(writer_stream)  
    );

    let items_stream = ReceiverStream::new(broker_rx);
    let writer_sink = PollSender::new(writer_tx);
    let broker_handle = tokio::task::spawn(
        broker.broker_loop(
            items_stream, 
            writer_sink,
            ctx,
            stop_broker,
            reader_handle,
            writer_handle
        )
    );

    (broker_handle, broker_tx)
}

/// Spawning a broker-reader-writer with `async-std` runtime
#[cfg(all(feature = "async-std", not(feature = "tokio")))]
pub fn spawn<B, R, W, BI, WI>(broker: B, reader: R, writer: W, bound: usize
) -> (async_std::task::JoinHandle<()>, Sender<BI>) 
where 
    B: Broker<Item = BI, WriterItem = WI> + Send + 'static,
    R: Reader<BrokerItem = BI> + Send + 'static,
    W: Writer<Item = WI> + Send + 'static,
    BI: Send + 'static,
    WI: Send + 'static,
{
    let (broker_tx, broker_rx) = bounded(bound);
    let (writer_tx, writer_rx) = bounded(bound);
    let (reader_stop, stop_reader) = bounded(1);
    let (broker_stop, stop_broker) = bounded(1);
    let ctx = Context {
        broker: broker_tx.clone(),
        broker_stop,
        reader_stop,
    };
    let ctx = Arc::new(ctx);

    let broker_sink = broker_tx.clone().into_sink();
    let reader_handle = async_std::task::spawn(
        reader.reader_loop(Arc::clone(&ctx), broker_sink, stop_reader)
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
            ctx,
            stop_broker,
            reader_handle,
            writer_handle
        )
    );

    (broker_handle, broker_tx)
}