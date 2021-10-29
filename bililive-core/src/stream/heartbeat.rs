use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Waker;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::ready;
use futures::{Sink, Stream};
use log::debug;

use crate::errors::Stream as StreamError;
use crate::packet::{Operation, Packet, Protocol};

use super::waker::WakerProxy;

pub struct HeartbeatStream<T, E> {
    /// underlying bilibili stream
    stream: T,
    /// waker proxy for tx, see WakerProxy for details
    tx_waker: Arc<WakerProxy>,
    /// last time when heart beat is sent
    last_hb: Option<Instant>,
    __marker: PhantomData<E>,
}

impl<T: Unpin, E> Unpin for HeartbeatStream<T, E> {}

impl<T, E> HeartbeatStream<T, E> {
    /// Add heartbeat response mechanism to the underlying bililive stream.
    ///
    /// You may want to use `connect` or `connect_with_retry` in [`connect`](crate::connect) module instead.
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            tx_waker: Arc::new(Default::default()),
            last_hb: None,
            __marker: PhantomData,
        }
    }
}

impl<T, E> Stream for HeartbeatStream<T, E>
where
    T: Stream<Item = Result<Packet, StreamError<E>>> + Sink<Packet, Error = StreamError<E>> + Unpin,
    E: std::error::Error,
{
    type Item = Result<Packet, StreamError<E>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // register current task to be waken on poll_ready
        self.tx_waker.rx(cx.waker());

        // ensure that all pending write op are completed
        ready!(self.as_mut().poll_ready(cx))?;

        // check whether we need to send heartbeat now.
        let now = Instant::now();
        let need_hb = self
            .last_hb
            .map_or(true, |last_hb| now - last_hb >= Duration::from_secs(30));

        if need_hb {
            // we need to send heartbeat, so push it into the sink
            debug!("sending heartbeat");
            self.as_mut()
                .start_send(Packet::new(Operation::HeartBeat, Protocol::Json, vec![]))?;

            // Update the time we sent the heartbeat.
            // It must be earlier than other non-blocking op so that heartbeat
            // won't be sent repeatedly.
            self.last_hb = Some(now);

            // Schedule current task to be waken in case there's no incoming
            // websocket message in a long time.
            #[cfg(feature = "tokio")]
            {
                let waker = cx.waker().clone();
                tokio::spawn(async {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    waker.wake();
                });
            }
            #[cfg(feature = "async-std")]
            {
                let waker = cx.waker().clone();
                async_std::task::spawn(async {
                    async_std::task::sleep(Duration::from_secs(30)).await;
                    waker.wake();
                });
            }
        }

        // ensure that heartbeat is sent
        ready!(self.as_mut().poll_flush(cx))?;

        Pin::new(&mut self.stream).poll_next(cx)
    }
}

impl<T, E> Sink<Packet> for HeartbeatStream<T, E>
where
    T: Sink<Packet, Error = StreamError<E>> + Unpin,
    E: std::error::Error,
{
    type Error = StreamError<E>;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // wake current task and stream task
        self.tx_waker.tx(cx.waker());
        let waker = Waker::from(self.tx_waker.clone());
        let mut cx = Context::from_waker(&waker);

        // poll the underlying websocket sink
        Pin::new(&mut self.stream).poll_ready(&mut cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        Pin::new(&mut self.stream).start_send(item)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // wake current task and stream task
        self.tx_waker.tx(cx.waker());
        let waker = Waker::from(self.tx_waker.clone());
        let mut cx = Context::from_waker(&waker);

        // poll the underlying websocket sink
        Pin::new(&mut self.stream).poll_flush(&mut cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // wake current task and stream task
        self.tx_waker.tx(cx.waker());
        let waker = Waker::from(self.tx_waker.clone());
        let mut cx = Context::from_waker(&waker);

        // poll the underlying websocket sink
        Pin::new(&mut self.stream).poll_close(&mut cx)
    }
}
