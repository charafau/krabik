//! Work that happens outside `update`, and how its result comes back.
//!
//! `update` runs on the UI thread and must return quickly. Anything slow (a
//! network request, reading a file) is described as a [`Command`] and
//! returned from `update`; the runtime runs it on another thread and delivers
//! the result as an ordinary message:
//!
//! ```ignore
//! fn update(&mut self, msg: Msg) -> Command<Msg> {
//!     match msg {
//!         Msg::Refresh => {
//!             self.loading = true;
//!             Command::perform(|| Msg::Loaded(fetch()))
//!         }
//!         Msg::Loaded(items) => {
//!             self.loading = false;
//!             self.items = items;
//!             Command::none()
//!         }
//!     }
//! }
//! ```
//!
//! Something that keeps happening while the application is in a certain
//! state, such as a clock, is a [`Subscription`] returned from
//! [`App::subscriptions`](crate::app::App::subscriptions).
//!
//! # How a result travels
//!
//! A message produced on a worker thread is put into the application's
//! [`Mailbox`] and the platform is asked to wake the UI thread. There the
//! session takes every waiting message, applies each with `update`, and
//! renders once.

use std::any::Any;
use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

/// A message on its way from a worker thread to the UI thread.
pub(crate) type Parcel = Box<dyn Any + Send>;

/// Where a job delivers the messages it produces.
type Sink = Arc<dyn Fn(Parcel) + Send + Sync>;

/// One piece of work. It is given the place to deliver its messages.
type Job = Box<dyn FnOnce(Sink) + Send>;

/// Work for the runtime to do after `update` returns.
///
/// A command is a description; nothing runs until `update` hands it back.
/// It is deliberately not `#[must_use]`: a test that calls `update` to
/// check the state it leaves is free to ignore the work it asked for.
pub struct Command<M> {
    jobs: Vec<Job>,
    message: PhantomData<fn() -> M>,
}

impl<M> Command<M> {
    /// Nothing to do. What most messages return.
    pub fn none() -> Self {
        Self {
            jobs: Vec::new(),
            message: PhantomData,
        }
    }

    /// Whether there is nothing to do.
    pub fn is_none(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Several commands as one. They run independently of each other.
    pub fn batch(commands: impl IntoIterator<Item = Command<M>>) -> Self {
        Self {
            jobs: commands
                .into_iter()
                .flat_map(|command| command.jobs)
                .collect(),
            message: PhantomData,
        }
    }

    pub(crate) fn into_jobs(self) -> Jobs {
        Jobs(self.jobs)
    }
}

impl<M: Send + 'static> Command<M> {
    /// Runs `work` on another thread and sends the message it returns.
    ///
    /// `work` may block: it has a thread to itself.
    pub fn perform(work: impl FnOnce() -> M + Send + 'static) -> Self {
        Self::job(move |sink| sink(Box::new(work())))
    }

    /// Drives `future` to completion on another thread and sends its output.
    ///
    /// The future is polled by a minimal executor that only sleeps between
    /// wake-ups. A future that needs a particular runtime (a `tokio` socket,
    /// for example) has to be spawned on that runtime; use
    /// [`Command::dispatcher`] to hand it a way back.
    pub fn future(future: impl Future<Output = M> + Send + 'static) -> Self {
        Self::job(move |sink| sink(Box::new(block_on(future))))
    }

    /// Runs `work` on another thread with a [`Dispatcher`] it can keep and
    /// use any number of times, from any thread: progress reports, a stream
    /// of events, or work handed to the application's own async runtime.
    pub fn dispatcher(work: impl FnOnce(Dispatcher<M>) + Send + 'static) -> Self {
        Self::job(move |sink| work(Dispatcher::new(sink)))
    }

    /// Lifts a child component's command into its parent's message type:
    /// `self.child.update(msg).map(Msg::Child)`.
    pub fn map<N: Send + 'static>(
        self,
        wrap: impl Fn(M) -> N + Send + Sync + 'static,
    ) -> Command<N> {
        let wrap = Arc::new(wrap);
        Command {
            jobs: self
                .jobs
                .into_iter()
                .map(|job| {
                    let wrap = wrap.clone();
                    Box::new(move |sink: Sink| {
                        job(Arc::new(move |parcel: Parcel| {
                            // Every job of a `Command<M>` delivers `M`.
                            if let Ok(message) = parcel.downcast::<M>() {
                                sink(Box::new(wrap(*message)));
                            }
                        }));
                    }) as Job
                })
                .collect(),
            message: PhantomData,
        }
    }

    fn job(job: impl FnOnce(Sink) + Send + 'static) -> Self {
        Self {
            jobs: vec![Box::new(job)],
            message: PhantomData,
        }
    }
}

impl<M> Default for Command<M> {
    fn default() -> Self {
        Self::none()
    }
}

impl<M> std::fmt::Debug for Command<M> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Command({} jobs)", self.jobs.len())
    }
}

/// The jobs of a command, with the message type erased.
pub(crate) struct Jobs(Vec<Job>);

impl Jobs {
    pub(crate) fn none() -> Self {
        Self(Vec::new())
    }
}

/// Sends messages to the application from any thread.
///
/// Obtained from [`Command::dispatcher`]. It can be cloned and kept for as
/// long as needed; messages sent after the application is gone are dropped.
pub struct Dispatcher<M> {
    sink: Sink,
    message: PhantomData<fn(M)>,
}

impl<M: Send + 'static> Dispatcher<M> {
    fn new(sink: Sink) -> Self {
        Self {
            sink,
            message: PhantomData,
        }
    }

    /// Delivers `message` to the application's `update` on the UI thread.
    pub fn send(&self, message: M) {
        (self.sink)(Box::new(message));
    }
}

impl<M> Clone for Dispatcher<M> {
    fn clone(&self) -> Self {
        Self {
            sink: self.sink.clone(),
            message: PhantomData,
        }
    }
}

/// Something that sends messages for as long as the application returns it
/// from [`App::subscriptions`](crate::app::App::subscriptions).
///
/// Subscriptions are compared by key after every message: one that appears
/// is started, one that disappears is stopped, and one that stays keeps
/// running undisturbed. They pause while the application is in the
/// background.
pub struct Subscription<M> {
    pub(crate) key: String,
    pub(crate) interval: Duration,
    pub(crate) tick: Arc<dyn Fn() -> Parcel + Send + Sync>,
    message: PhantomData<fn() -> M>,
}

impl<M: Send + 'static> Subscription<M> {
    /// Sends the message `message` returns every `interval`, first after one
    /// interval has passed.
    ///
    /// `key` identifies the subscription among the ones the application
    /// returns. Changing the interval under the same key restarts it.
    pub fn every(
        key: impl Into<String>,
        interval: Duration,
        message: impl Fn() -> M + Send + Sync + 'static,
    ) -> Self {
        Self {
            key: key.into(),
            interval,
            tick: Arc::new(move || Box::new(message())),
            message: PhantomData,
        }
    }

    /// Lifts a child component's subscription into its parent's messages.
    pub fn map<N: Send + 'static>(
        self,
        wrap: impl Fn(M) -> N + Send + Sync + 'static,
    ) -> Subscription<N> {
        let tick = self.tick;
        Subscription {
            key: self.key,
            interval: self.interval,
            tick: Arc::new(move || match tick().downcast::<M>() {
                Ok(message) => Box::new(wrap(*message)),
                // Not reachable: `tick` was built from a function to `M`.
                Err(parcel) => parcel,
            }),
            message: PhantomData,
        }
    }
}

impl<M> Subscription<M> {
    pub(crate) fn erase(self) -> Subscription<()> {
        Subscription {
            key: self.key,
            interval: self.interval,
            tick: self.tick,
            message: PhantomData,
        }
    }
}

/// Runs jobs and timers. The real one uses threads; tests run jobs inline
/// and fire timers by hand.
pub(crate) trait Executor: Send + Sync {
    /// Runs `job` without blocking the caller.
    fn spawn(&self, job: Box<dyn FnOnce() + Send>);
    /// Calls `tick` every `interval` until `stopped` is set.
    fn every(&self, interval: Duration, stopped: Arc<AtomicBool>, tick: Box<dyn Fn() + Send>);
}

/// One thread per job and per timer.
pub(crate) struct Threads;

impl Executor for Threads {
    fn spawn(&self, job: Box<dyn FnOnce() + Send>) {
        let spawned = std::thread::Builder::new()
            .name("krabik-command".into())
            .spawn(job);
        if let Err(error) = spawned {
            crate::log::error(&format!("could not start a thread for a command: {error}"));
        }
    }

    fn every(&self, interval: Duration, stopped: Arc<AtomicBool>, tick: Box<dyn Fn() + Send>) {
        // A zero interval would spin; a frame is the shortest useful period.
        let interval = interval.max(Duration::from_millis(16));
        let spawned = std::thread::Builder::new()
            .name("krabik-timer".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(interval);
                    if stopped.load(Ordering::Relaxed) {
                        break;
                    }
                    tick();
                }
            });
        if let Err(error) = spawned {
            crate::log::error(&format!("could not start a timer thread: {error}"));
        }
    }
}

/// The messages waiting for the UI thread, and the way to wake it.
///
/// The mailbox belongs to the application, not to the session that shows
/// it: when Android recreates an activity, work started before continues to
/// deliver here and the next session picks the messages up.
#[derive(Default)]
pub(crate) struct Mailbox {
    queue: Mutex<VecDeque<Parcel>>,
    /// Asks the platform to call the session's drain on the UI thread.
    waker: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    /// Set while a wake-up is on its way, so a burst of messages asks once.
    waking: AtomicBool,
}

impl Mailbox {
    /// Queues a message and wakes the UI thread.
    pub(crate) fn post(&self, parcel: Parcel) {
        self.queue.lock().expect("mailbox").push_back(parcel);
        self.wake();
    }

    /// Asks for a drain unless one is already on its way.
    pub(crate) fn wake(&self) {
        let waker = self.waker.lock().expect("mailbox waker").clone();
        if let Some(waker) = waker
            && !self.waking.swap(true, Ordering::AcqRel)
        {
            waker();
        }
    }

    /// Takes every waiting message. Called on the UI thread.
    pub(crate) fn take(&self) -> Vec<Parcel> {
        // Cleared first: a message posted from here on asks for a new wake.
        self.waking.store(false, Ordering::Release);
        self.queue.lock().expect("mailbox").drain(..).collect()
    }

    /// Connects the mailbox to the session that now shows the application,
    /// or disconnects it with `None`.
    pub(crate) fn set_waker(&self, waker: Option<Arc<dyn Fn() + Send + Sync>>) {
        *self.waker.lock().expect("mailbox waker") = waker;
        self.waking.store(false, Ordering::Release);
        if !self.queue.lock().expect("mailbox").is_empty() {
            self.wake();
        }
    }
}

/// Starts the jobs of a command; each delivers into `mailbox`.
pub(crate) fn run(jobs: Jobs, mailbox: &Arc<Mailbox>, executor: &dyn Executor) {
    for job in jobs.0 {
        let mailbox = mailbox.clone();
        executor.spawn(Box::new(move || {
            job(Arc::new(move |parcel| mailbox.post(parcel)));
        }));
    }
}

/// Drives a future on the current thread, sleeping between wake-ups.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    struct Signal {
        woken: Mutex<bool>,
        changed: Condvar,
    }

    impl Wake for Signal {
        fn wake(self: Arc<Self>) {
            *self.woken.lock().expect("signal") = true;
            self.changed.notify_one();
        }
    }

    let signal = Arc::new(Signal {
        woken: Mutex::new(false),
        changed: Condvar::new(),
    });
    let waker = Waker::from(signal.clone());
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        let mut woken = signal.woken.lock().expect("signal");
        while !*woken {
            woken = signal.changed.wait(woken).expect("signal");
        }
        *woken = false;
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! An executor a test controls.

    use super::*;

    /// Runs jobs at once on the calling thread and keeps timers for the
    /// test to fire.
    #[derive(Default)]
    pub(crate) struct Inline {
        timers: Mutex<Vec<Timer>>,
    }

    struct Timer {
        interval: Duration,
        stopped: Arc<AtomicBool>,
        tick: Box<dyn Fn() + Send>,
    }

    impl Inline {
        /// The intervals of the timers that are running.
        pub(crate) fn running(&self) -> Vec<Duration> {
            self.timers
                .lock()
                .unwrap()
                .iter()
                .filter(|timer| !timer.stopped.load(Ordering::Relaxed))
                .map(|timer| timer.interval)
                .collect()
        }

        /// Lets every running timer fire once.
        pub(crate) fn fire(&self) {
            for timer in self.timers.lock().unwrap().iter() {
                if !timer.stopped.load(Ordering::Relaxed) {
                    (timer.tick)();
                }
            }
        }
    }

    impl Executor for Inline {
        fn spawn(&self, job: Box<dyn FnOnce() + Send>) {
            job();
        }

        fn every(&self, interval: Duration, stopped: Arc<AtomicBool>, tick: Box<dyn Fn() + Send>) {
            self.timers.lock().unwrap().push(Timer {
                interval,
                stopped,
                tick,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::Inline;
    use super::*;

    fn delivered(command: Command<u32>) -> Vec<u32> {
        let mailbox = Arc::new(Mailbox::default());
        run(command.into_jobs(), &mailbox, &Inline::default());
        mailbox
            .take()
            .into_iter()
            .map(|parcel| *parcel.downcast::<u32>().unwrap())
            .collect()
    }

    #[test]
    fn perform_delivers_what_the_work_returns() {
        assert_eq!(delivered(Command::perform(|| 7)), [7]);
        assert!(delivered(Command::none()).is_empty());
    }

    #[test]
    fn a_batch_runs_every_command() {
        let command = Command::batch([Command::perform(|| 1), Command::perform(|| 2)]);
        assert_eq!(delivered(command), [1, 2]);
    }

    #[test]
    fn map_lifts_a_child_message() {
        let child: Command<u8> = Command::perform(|| 3);
        assert_eq!(delivered(child.map(|value| u32::from(value) * 10)), [30]);
    }

    #[test]
    fn a_dispatcher_can_send_more_than_once() {
        let command = Command::dispatcher(|dispatcher: Dispatcher<u32>| {
            let other = dispatcher.clone();
            dispatcher.send(1);
            other.send(2);
        });
        assert_eq!(delivered(command), [1, 2]);
    }

    #[test]
    fn a_future_is_driven_to_completion_across_threads() {
        /// Ready the second time it is polled, woken from another thread.
        struct Later(bool);
        impl Future for Later {
            type Output = u32;
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
                if self.0 {
                    return Poll::Ready(9);
                }
                self.0 = true;
                let waker = cx.waker().clone();
                std::thread::spawn(move || waker.wake());
                Poll::Pending
            }
        }
        assert_eq!(delivered(Command::future(Later(false))), [9]);
        assert_eq!(delivered(Command::future(async { 4 })), [4]);
    }

    #[test]
    fn a_burst_of_messages_wakes_the_ui_thread_once() {
        let mailbox = Mailbox::default();
        let wakes = Arc::new(Mutex::new(0));
        let counter = wakes.clone();
        mailbox.set_waker(Some(Arc::new(move || *counter.lock().unwrap() += 1)));

        mailbox.post(Box::new(1_u32));
        mailbox.post(Box::new(2_u32));
        assert_eq!(*wakes.lock().unwrap(), 1);

        // After a drain the next message asks again.
        assert_eq!(mailbox.take().len(), 2);
        mailbox.post(Box::new(3_u32));
        assert_eq!(*wakes.lock().unwrap(), 2);
    }

    #[test]
    fn messages_wait_for_a_session_to_connect() {
        let mailbox = Mailbox::default();
        mailbox.post(Box::new(1_u32));

        let wakes = Arc::new(Mutex::new(0));
        let counter = wakes.clone();
        mailbox.set_waker(Some(Arc::new(move || *counter.lock().unwrap() += 1)));
        assert_eq!(*wakes.lock().unwrap(), 1);
        assert_eq!(mailbox.take().len(), 1);
    }

    #[test]
    fn a_subscription_makes_a_message_per_tick() {
        let subscription =
            Subscription::every("tick", Duration::from_secs(1), || 5_u8).map(u32::from);
        assert_eq!(subscription.key, "tick");
        assert_eq!(*(subscription.tick)().downcast::<u32>().unwrap(), 5);
    }
}
