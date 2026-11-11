//! Threads that each own a connection and run the work sent to them.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use futures_channel::oneshot;
use rusqlite::Connection;

use super::{Error, connection};

type Job = Box<dyn FnOnce(&mut Connection) + Send>;

/// `size` threads sharing one queue of work.
pub(super) struct Pool {
    sender: Sender<Job>,
    /// Each thread signals here once its connection has closed.
    closed: Vec<oneshot::Receiver<()>>,
}

impl Pool {
    /// Starts `size` threads, each opening its own connection with `open`.
    /// Fails if any connection fails to open. Whenever `statistics` changes,
    /// each connection reloads the planner's statistics before its next job.
    pub async fn open<F>(size: usize, statistics: Arc<AtomicU64>, open: F) -> Result<Pool, Error>
    where
        F: Fn() -> Result<Connection, Error> + Send + Sync + 'static,
    {
        let open = Arc::new(open);
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut opened = Vec::with_capacity(size);
        let mut closed = Vec::with_capacity(size);
        for index in 0..size {
            let (opened_sender, opened_receiver) = oneshot::channel();
            let (closed_sender, closed_receiver) = oneshot::channel();
            let open = Arc::clone(&open);
            let receiver = Arc::clone(&receiver);
            let statistics = Arc::clone(&statistics);
            thread::Builder::new()
                .name(format!("mariner-db-{index}"))
                .spawn(move || {
                    serve(&*open, &receiver, &statistics, opened_sender, closed_sender)
                })?;
            opened.push(opened_receiver);
            closed.push(closed_receiver);
        }
        for opened in opened {
            opened.await.map_err(|_| Error::Closed)??;
        }
        Ok(Pool { sender, closed })
    }

    /// Runs `work` on the next free connection and returns its result.
    pub async fn run<T, F>(&self, work: F) -> Result<T, Error>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let job: Job = Box::new(move |connection| {
            let result = catch_unwind(AssertUnwindSafe(|| work(connection)))
                .map_err(|_| Error::Panicked)
                .and_then(|result| result.map_err(Error::from));
            let _ = sender.send(result);
        });
        self.sender.send(job).map_err(|_| Error::Closed)?;
        receiver.await.map_err(|_| Error::Closed)?
    }

    /// Stops taking work, and waits until every connection has closed.
    pub async fn close(self) {
        drop(self.sender);
        for closed in self.closed {
            let _ = closed.await;
        }
    }
}

/// A pool thread: opens its connection, reports whether that worked, then
/// runs jobs until the pool is closed or dropped.
fn serve(
    open: &dyn Fn() -> Result<Connection, Error>,
    receiver: &Mutex<Receiver<Job>>,
    statistics: &AtomicU64,
    opened_sender: oneshot::Sender<Result<(), Error>>,
    closed_sender: oneshot::Sender<()>,
) {
    let mut loaded = statistics.load(Ordering::Acquire);
    let mut connection = match open() {
        Ok(connection) => {
            let _ = opened_sender.send(Ok(()));
            connection
        }
        Err(error) => {
            let _ = opened_sender.send(Err(error));
            return;
        }
    };
    loop {
        let job = match receiver.lock() {
            Ok(receiver) => receiver.recv(),
            Err(_) => break,
        };
        let Ok(job) = job else { break };
        let current = statistics.load(Ordering::Acquire);
        if current != loaded {
            // A connection reads the planner's statistics when it opens, and
            // a read-only one can't be told to reload them.
            if let Ok(reopened) = open() {
                connection::close(std::mem::replace(&mut connection, reopened));
            }
            loaded = current;
        }
        job(&mut connection);
    }
    connection::close(connection);
    let _ = closed_sender.send(());
}
