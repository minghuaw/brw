//! Writer trait definition

use async_trait::async_trait;
use futures_util::{
    stream::{Stream, StreamExt},
};

use super::Running;

/// Writer of the broker-reader-writer pattern
#[async_trait]
pub trait Writer: Sized {
    /// Item to receive
    type Item: Send + 'static;
    /// Ok result from `op`
    type Ok: Send;
    /// Ok result from `op`
    type Error: std::error::Error + Send;

    /// The operation to perform
    async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>, ()>;

    /// Handles the result of each op
    /// 
    /// Returns a `None` to stop the whole loop
    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<(), ()> {
        if let Err(_err) = res {
            #[cfg(feature = "debug")]
            log::error!("{:?}", _err);
        }
        Running::Continue(())
    }
    /// Runs the operation in a loop
    async fn writer_loop<S>(mut self, mut items: S) 
    where 
        S: Stream<Item = Self::Item> + Send + Unpin
    {
        while let Some(item) = items.next().await {
            match self.op(item).await {
                Running::Continue(res) => {
                    match <Self as Writer>::handle_result(res).await {
                        Running::Continue(_) => { },
                        Running::Stop(_) => break
                    }
                },
                Running::Stop(_) => break
            }
        }

        #[cfg(feature = "debug")]
        log::debug!("Exiting writer loop");
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use futures_util::sink::SinkExt;
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    use crate::{Writer, Running};
    // Simply print out receive items
    struct TestWriter { }

    #[derive(Debug)]
    enum TestWriterItem {
        Foo(String),
        Bar(i32)
    }

    #[async_trait]
    impl super::Writer for TestWriter {
        type Item = TestWriterItem;
        type Ok = ();
        type Error = std::io::Error;

        async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>, ()> {
            println!("{:?}", item);

            Running::Continue(Ok(()))
        }

        async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<(), ()> {
            if let Err(err) = res {
                println!("{:?}", err);
            }
            Running::Continue(())
        }
    }

    #[tokio::test]
    async fn test_writer() {
        let bound = 100;
        let w = TestWriter{ };
        let (tx, rx) = tokio::sync::mpsc::channel(bound);
        let mut tx = PollSender::new(tx);
        let handle = tokio::task::spawn(w.writer_loop(ReceiverStream::new(rx)));
        
        for i in 0..10 {
            tx.send(TestWriterItem::Foo(i.to_string())).await
                .unwrap();
            tx.send(TestWriterItem::Bar(i)).await
                .unwrap();
        }
        drop(tx);

        handle.await.unwrap();
    }
}