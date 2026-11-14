# 11. Commands and the mailbox

Chapters 1 to 10 describe a loop that runs entirely on the UI thread: an event arrives, the
application updates, every host renders. This chapter describes the one part of the framework
that uses other threads: work started by `update`, timers, and the path their results take back
to the UI thread.

The code is in `rust/src/command.rs` (the types and the mailbox), `rust/src/host/session.rs`
(running and draining) and two short pieces per platform (the wake-up). The application-facing
side is in [tutorial chapter 11](../tutorial/11-background-work-and-timers.md).

## The shape of the problem

Three facts decide the design.

1. **The session is only ever used on the UI thread.** It holds JNI references on Android and
   UIKit objects on iOS. Nothing in it may be touched from a worker thread.
2. **A message is `Box<dyn Any>`** (`Message` in `rust/src/message.rs`), and that is not `Send`.
   Handlers produce it on the UI thread and it never leaves.
3. **A session can end while work is running.** On Android an activity is destroyed and
   recreated on a theme change; the application object is carried over (`RetainedApp`, see the
   Android shell chapter), the session is not.

So a worker thread needs somewhere to put a result that is not the session, in a form that can
cross threads, and that outlives the session. That place is the mailbox.

## The pieces

```text
   UI thread                                   worker thread
   ---------                                   -------------
   Session::apply(message)
     app.update(message) -> Jobs
     command::run(jobs, mailbox, executor) --> job(sink)
                                                 work()            the application's closure
                                                 sink(Box::new(result))
                                               Mailbox::post(parcel)
                                                 queue.push_back
                                                 wake()  -- once per burst -->  platform
   platform posts to the UI thread  <------------------------------------------
   Session::drain
     mailbox.take()
     for each: apply(message)
     render_all
```

### `Command<M>` and its jobs

```rust,ignore
// rust/src/command.rs
pub(crate) type Parcel = Box<dyn Any + Send>;
type Sink = Arc<dyn Fn(Parcel) + Send + Sync>;
type Job = Box<dyn FnOnce(Sink) + Send>;

pub struct Command<M> {
    jobs: Vec<Job>,
    message: PhantomData<fn() -> M>,
}
```

A command is a list of jobs. A job is a closure that receives a *sink*, the function to call
with each message it produces. The message type `M` appears only in `PhantomData`: the jobs
themselves are already erased.

That choice is what lets `Component::Msg` stay `'static` without requiring `Send`. Look at where
the bound sits:

```rust,ignore
impl<M> Command<M> {
    pub fn none() -> Self { ... }                    // any M
    pub fn batch(...) -> Self { ... }                // any M
}

impl<M: Send + 'static> Command<M> {
    pub fn perform(work: impl FnOnce() -> M + Send + 'static) -> Self {
        Self::job(move |sink| sink(Box::new(work())))
    }
    ...
}
```

The boxing into a `Parcel` happens inside the closure that `perform` builds, where `M: Send` is
known. An application whose messages are not `Send` can still implement `update`; it just cannot
call `perform`. If the erasure happened later, in the `AnyApp` adapter, every application would
need `Msg: Send`.

`Parcel` is `Box<dyn Any + Send>`, the `Send` sibling of `Message`. Converting a parcel to a
message on the UI thread is a plain unsizing coercion; `Session::apply` takes a `Message` and is
called with a `Parcel`.

The other constructors are the same job with a different body:

- `future(f)`: `sink(Box::new(block_on(f)))`. `block_on` is thirty lines at the bottom of the
  file: a `Waker` built from an `Arc<Signal>` that sets a flag and notifies a condition
  variable, and a loop that polls and then sleeps until the flag is set. It brings no dependency
  and no reactor, which is why futures that need a specific runtime cannot run on it.
- `dispatcher(f)`: `f(Dispatcher::new(sink))`. The `Dispatcher<M>` is the sink with the message
  type put back, so the application gets a typed `send(M)`.

`map` wraps the sink instead of the result:

```rust,ignore
pub fn map<N: Send + 'static>(self, wrap: impl Fn(M) -> N + Send + Sync + 'static) -> Command<N> {
    let wrap = Arc::new(wrap);
    Command {
        jobs: self.jobs.into_iter().map(|job| {
            let wrap = wrap.clone();
            Box::new(move |sink: Sink| {
                job(Arc::new(move |parcel: Parcel| {
                    if let Ok(message) = parcel.downcast::<M>() {
                        sink(Box::new(wrap(*message)));
                    }
                }));
            }) as Job
        }).collect(),
        message: PhantomData,
    }
}
```

The child's job is handed a sink that downcasts to the child's type, wraps, and passes on. This
is the mirror image of `Element::map` in chapter 2, which wraps the *handlers* of a child's view.

`Command` is deliberately not `#[must_use]`. Inside `update` the return type already forces the
command to be returned. The only place a command is routinely dropped is a unit test that calls
`update` to inspect the state, and a warning there would be noise.

### The erased trait

```rust,ignore
// rust/src/app.rs
pub(crate) trait AnyApp: Send {
    fn update(&mut self, message: Message) -> Jobs;
    fn subscriptions(&self) -> Vec<Subscription<()>>;
    ...
}

fn update(&mut self, message: Message) -> Jobs {
    match message.downcast::<A::Msg>() {
        Ok(message) => Component::update(self, *message).into_jobs(),
        Err(_) => Jobs::none(),
    }
}
```

`Jobs` is the command without its type parameter. Nothing else changes in the adapter.

### The mailbox

```rust,ignore
// rust/src/command.rs
pub(crate) struct Mailbox {
    queue: Mutex<VecDeque<Parcel>>,
    waker: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    waking: AtomicBool,
}
```

- `post(parcel)` pushes and calls `wake()`.
- `wake()` calls the waker unless `waking` is already set. A burst of results asks the platform
  once.
- `take()` clears `waking` first and then drains the queue. The order matters: a message posted
  after the flag is cleared asks for a new wake-up even if `take` has not finished, so a message
  can be woken for twice but never zero times.
- `set_waker(Some(..))` connects a session, and wakes at once if messages are already waiting.
  `set_waker(None)` disconnects it.

**The mailbox belongs to the application, not to the session.** `Running` in `rust/src/app.rs`
pairs the two:

```rust,ignore
pub(crate) struct Running {
    pub(crate) app: Box<dyn AnyApp>,
    pub(crate) mailbox: Arc<Mailbox>,
}
```

`create_app()` returns a `Running`. `Session::into_app` hands one back when an activity is being
recreated, and Android keeps it in `RETAINED_APPS` until the next `nativeCreate` picks it up.
Worker threads hold an `Arc<Mailbox>` through their sink, so they keep delivering while no
session exists; the next session's `set_waker` finds the queue non-empty and asks for a drain.
`work_outlives_the_session_that_started_it` tests exactly that sequence.

### Running and draining

```rust,ignore
// rust/src/host/session.rs
fn apply(&mut self, message: Message) {
    let jobs = self.app.update(message);
    command::run(jobs, &self.mailbox.0, self.executor.as_ref());
}

pub(crate) fn drain(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
    let parcels = self.mailbox.0.take();
    if parcels.is_empty() {
        return Ok(());
    }
    for parcel in parcels {
        self.apply(parcel);
    }
    self.render_all(cx)
}
```

`apply` is the only place `AnyApp::update` is called. Every path that used to call it
(`node_event`, `back`, the chrome events, a dismissed dialog or sheet, a native pop) goes
through it, so a command returned in answer to any of them is run.

`drain` applies everything that is waiting and renders once. Ten results that arrive together
cost one render, not ten.

### The executor

```rust,ignore
pub(crate) trait Executor: Send + Sync {
    fn spawn(&self, job: Box<dyn FnOnce() + Send>);
    fn every(&self, interval: Duration, stopped: Arc<AtomicBool>, tick: Box<dyn Fn() + Send>);
}
```

`Threads`, the real one, starts a named thread per job (`krabik-command`) and per timer
(`krabik-timer`). There is no pool: a mobile app starts a handful of requests, and a thread per
request keeps one slow call from delaying another. A failure to start a thread is logged; the
message is then never produced.

`testing::Inline` is the test double. `spawn` runs the job immediately on the calling thread, so
a session test sees the result in the mailbox as soon as `update` returns. `every` stores the
timer; the test calls `fire()` to make time pass. `Session::with_executor` is how a test passes
it in; `Session::new` uses `Threads`.

## Subscriptions

```rust,ignore
pub struct Subscription<M> {
    pub(crate) key: String,
    pub(crate) interval: Duration,
    pub(crate) tick: Arc<dyn Fn() -> Parcel + Send + Sync>,
    message: PhantomData<fn() -> M>,
}
```

Like a command, a subscription is erased when it is built: `tick` already returns a parcel.

`Session::sync_subscriptions` runs at the start of every `render_all`, before the hosts' chrome
is mirrored, and from `set_active`:

```rust,ignore
let wanted = if self.active { self.app.subscriptions() } else { Vec::new() };
self.timers.0.retain(|key, (interval, stopped)| {
    let keep = wanted.iter().any(|s| s.key == *key && s.interval == *interval);
    if !keep { stopped.store(true, Ordering::Relaxed); }
    keep
});
for subscription in wanted {
    if self.timers.0.contains_key(&subscription.key) { continue; }
    // start it: executor.every(interval, stopped, tick -> mailbox.post)
}
```

It is a diff by key, the same idea as the navigation stack and the chrome in chapter 8: the
application returns what it wants, the session compares it with what is running.

Stopping is cooperative. Each timer has an `AtomicBool`. Setting it does not interrupt the
thread; the thread notices after its current sleep and exits. The tick closure checks the flag
again before posting, so a tick that was due at the moment of stopping is dropped.

`set_active(false)` makes `wanted` empty, which stops everything; `set_active(true)` starts the
application's list again. The platforms call it from `onStart`/`onStop` and from the
foreground/background notifications.

## Ending a session

```rust,ignore
struct Link(Arc<Mailbox>);
impl Drop for Link { fn drop(&mut self) { self.0.set_waker(None); } }

struct Timers(HashMap<String, (Duration, Arc<AtomicBool>)>);
impl Drop for Timers { fn drop(&mut self) { /* set every flag */ } }
```

The session holds its mailbox through `Link` and its timers through `Timers`. Both clean up in
`Drop`, so every way a session can end (destroyed, or consumed by `into_app`) stops its timers
and stops the mailbox from waking a handle that no longer exists. Commands already running are
not cancelled: they finish and post, and the parcel waits.

## The wake-up on each platform

The waker is the only platform-specific part. It must be callable from any thread and must end
in `Session::drain` on the UI thread.

### iOS

```rust,ignore
// rust/src/ios/entry.rs, in krabik_ios_create
session.set_waker(std::sync::Arc::new(move || schedule(move || drain(handle))));
```

`schedule` is `DispatchQueue::main().exec_async` (`rust/src/ios/mod.rs`), the same hop every
widget event and every finished image download already uses. `drain(handle)` looks the session
up by handle; if the scene is gone it finds nothing.

### Android

There is no way to post to the main looper from Rust without Java, so the hop goes through
Kotlin:

```rust,ignore
// rust/src/android/entry.rs, in native_create
let vm = env.get_java_vm()?;
core.set_waker(Arc::new(move || {
    let posted = vm.attach_current_thread(|env| -> JniResult<()> {
        let ids = jni_handles(env)?;
        ids.post_wake.call_void(env, &ids.rust_root_view, &[JValue::Long(handle)])
    });
    ...
}));
```

```kotlin
// shared/android/src/main/java/dev/krabik/ui/RustRootView.kt
private val mainHandler = Handler(Looper.getMainLooper())

@JvmStatic
fun postWake(handle: Long) {
    mainHandler.post { nativeOnWake(handle) }
}
```

The worker thread attaches to the JVM (the measurer in `rust/src/android/measure.rs` already
does this), calls the static `postWake`, and returns. The class and method ids were resolved on
the main thread in `Jni::load`; a worker thread could not resolve an application class itself,
because `FindClass` on a thread Java did not start uses the system class loader.

`nativeOnWake` then runs on the main thread:

```rust,ignore
let Some(mut states) = lock_states_for_event() else {
    // Only reachable if a render caused this call synchronously.
    // The messages must not be lost, so ask again.
    return ids.post_wake.call_void(env, &ids.rust_root_view, &[JValue::Long(handle)]);
};
match states.get_mut(&handle) {
    Some(session) => session.core.drain(env),
    None => Ok(()),
}
```

Chapter 2 of the platform part explains `lock_states_for_event`: event entry points use
`try_lock` and drop the event when a render holds the lock, because that can only be an echo of
the render itself. A wake-up is not an echo and must not be dropped, so on contention it posts
itself again.

## What is not there

- **No cancellation.** A command cannot be stopped once started. Its result arrives and
  `update` decides whether it still matters.
- **No thread pool and no limit.** An application that starts a thousand commands starts a
  thousand threads.
- **No timer precision.** A timer sleeps for its interval between ticks, and a tick waits for
  the UI thread.
- **Panics in work are not reported.** The thread ends and nothing is posted.
- **iOS does not retain the application across scene destruction**, so the "work outlives the
  session" path only matters on Android today.

## Where this lives

- `rust/src/command.rs`: everything in this chapter that is not session code, and its tests.
- `rust/src/app.rs`: `Component::update`, `App::subscriptions`, `AnyApp`, `Running`.
- `rust/src/host/session.rs`: `apply`, `drain`, `set_waker`, `set_active`,
  `sync_subscriptions`, `Link`, `Timers`, and the tests from
  `the_result_of_a_command_comes_back_as_a_message` on.
- `rust/src/android/entry.rs`: the waker in `native_create`, `native_on_wake`,
  `native_set_active`. `rust/src/android/handles.rs`: `post_wake`.
- `rust/src/ios/entry.rs`: the waker in `krabik_ios_create`, `drain`, `krabik_ios_set_active`.
- `shared/android/.../RustRootView.kt`: `postWake`, `setActive`.
  `shared/android/.../KrabikActivity.kt`: `onStart`, `onStop`.
- `shared/ios/KrabikHostController.swift`: `becameActive`, `becameInactive`.

Back to the [index](README.md).
