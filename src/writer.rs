//! Writer trait definition

use async_trait::async_trait;
use futures::{
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
    async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>>;

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
    async fn writer_loop<S>(mut self, mut items: S) 
    where 
        S: Stream<Item = Self::Item> + Send + Unpin
    {
        while let Some(item) = items.next().await {
            match self.op(item).await {
                Running::Continue(res) => {
                    match <Self as Writer>::handle_result(res).await {
                        Running::Continue(_) => { },
                        Running::Stop => break
                    }
                },
                Running::Stop => break
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use futures::sink::SinkExt;

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

        async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>> {
            println!("{:?}", item);

            Running::Continue(Ok(()))
        }

        async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
            if let Err(err) = res {
                println!("{:?}", err);
            }
            Running::Continue(())
        }
    }

    #[tokio::test]
    async fn test_writer() {
        let w = TestWriter{ };
        let (tx, rx) = flume::unbounded();
        let mut tx = tx.into_sink();
        let handle = tokio::task::spawn(w.writer_loop(rx.into_stream()));
        
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