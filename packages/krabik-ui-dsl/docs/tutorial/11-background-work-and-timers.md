# 11. Background work and timers

Everything so far happened instantly: a tap arrived, `update` changed some state, the screen was
redrawn. Real apps also wait for things: a server, a file, the clock. This chapter explains how
an app does that without freezing, using the two tools the framework has for it: a `Command`
returned from `update`, and a `Subscription` returned from `App::subscriptions`.

Afterwards you will know why `update` returns a value, how a result gets from another thread
back into your state, and how to test code that uses either.

## The problem

`update` runs on the UI thread. That is the thread that draws the screen and handles touches.
While `update` runs, nothing else on that thread does:

```rust,ignore
// Do not do this.
Msg::Refresh => {
    self.notes = fetch_notes_from_server();   // two seconds; the app is frozen for all of them
}
```

Moving the call to another thread fixes the freeze and creates a new problem: the other thread
cannot touch `self`. The state belongs to the UI thread, and the framework only redraws after a
message. So the result has to come back **as a message**. That is the whole design:

1. `update` describes the work and returns at once.
2. The framework runs the work on another thread.
3. The work produces a message.
4. The message is delivered to `update` on the UI thread, like a tap would be.

## A command is the description of that work

`Component::update` returns a `Command<Msg>` (`rust/src/app.rs`, `rust/src/command.rs`):

```rust,ignore
fn update(&mut self, msg: Self::Msg) -> Command<Self::Msg>;
```

Most messages start no work and return `Command::none()`. That is why every `update` in this
tutorial ends with that line. The ones that do start work return it:

```rust,ignore
Msg::Refresh => {
    self.loading = true;
    return Command::perform(|| Msg::Loaded(fetch_notes_from_server()));
}
Msg::Loaded(notes) => {
    self.loading = false;
    self.notes = notes;
}
```

Read `Command::perform(work)` as "run `work` on another thread and send me what it returns".
The closure may block for as long as it likes; it has a thread to itself. It cannot borrow from
`self` (the closure must be `'static`), so copy or clone what it needs before building it.

Two things are worth noticing.

**The state change and the request sit together.** `self.loading = true` and the command are in
the same match arm. The spinner cannot be forgotten, because the place that starts the request
is the place that records it was started.

**A command is a value, not an action.** Building one runs nothing. It only runs if `update`
returns it. That is what keeps `update` testable: a test can call it and look at the state
without any thread being started (more on this below).

### What a message must be

A message that crosses threads has to be `Send`. `Command::perform` requires `M: Send + 'static`,
so an app that uses commands needs a `Msg` made of ordinary data: strings, numbers, vectors,
your own structs. An `Rc` inside a message would not compile. Apps that never build a command
are not affected; `Command::none()` has no such requirement.

### The other ways to build a command

| Constructor | Use it for |
|---|---|
| `Command::none()` | Nothing to do. |
| `Command::perform(closure)` | One blocking piece of work with one result. |
| `Command::future(future)` | An `async` block. It is polled on its own thread by a minimal executor. |
| `Command::dispatcher(closure)` | Work that reports more than once (progress), or that you hand to your own async runtime. |
| `Command::batch([a, b])` | Several commands at once. They run independently. |

`Command::future` has a limit you should know before reaching for it. The built-in executor only
sleeps until the future wakes it. That is enough for futures that are driven by their own
threads or by plain channels. A future that needs a specific runtime, a `tokio` socket for
example, panics or hangs without that runtime. For those, start your own runtime once and use a
dispatcher:

```rust,ignore
Msg::Refresh => {
    let runtime = self.runtime.clone();              // your tokio::runtime::Handle
    return Command::dispatcher(move |dispatcher| {
        runtime.spawn(async move {
            let notes = fetch().await;
            dispatcher.send(Msg::Loaded(notes));      // from any thread, any number of times
        });
    });
}
```

A `Dispatcher<Msg>` can be cloned, kept, and used from any thread. Each `send` becomes one
message. The framework does not ship an HTTP client; use whichever crate suits the app.

## Work that starts with the app

Some work has no message to start it: the first screen needs its data before the user has
touched anything. `App::init` returns the command for that:

```rust,ignore
impl App for Notes {
    fn init(&mut self) -> Command<Msg> {
        self.loading = true;
        Command::perform(|| Msg::Loaded(fetch_notes()))
    }
}
```

It is called once for every instance of the app, before the first render, so a change it makes
to the state (`loading` here) is already on the first screen. The result arrives through
`update` like any other. An Android activity that is recreated by a rotation or a theme change
keeps the same instance, so `init` does not run a second time.

## Child components

A child's `update` returns a `Command` of the child's message type. The parent lifts it the same
way it lifts the child's view:

```rust,ignore
// view:    self.composer.view().map(Msg::Composer)
// update:
Msg::Composer(msg) => return self.composer.update(msg).map(Msg::Composer),
```

`Command::map` wraps every message the child's work produces, so a result that started in the
composer arrives at the app as `Msg::Composer(..)` and is forwarded down again. The showcase
does this for its checklist (`Msg::Checklist` in `example/rust/src/lib.rs`).

## A subscription is something that keeps happening

A command happens once. A clock, a poll every thirty seconds, a countdown: these keep sending
messages for as long as the app is in a certain state. Describing them as "start" and "stop"
commands would bring back exactly the bookkeeping the framework exists to remove. Instead the
app says, from its state, what should be running right now:

```rust,ignore
fn subscriptions(&self) -> Vec<Subscription<Msg>> {
    if self.clock_on {
        vec![Subscription::every("clock", Duration::from_secs(1), || Msg::Tick)]
    } else {
        Vec::new()
    }
}
```

This is `view()` for time. After every message the framework calls `subscriptions()` and
compares the result with what is running, by key:

- a key that is new is started;
- a key that is gone is stopped;
- a key that is still there keeps running undisturbed, so returning the same subscription again
  and again does not restart its timer;
- the same key with another interval is restarted.

There is nothing to cancel by hand. Turning `clock_on` off is how the clock stops.

**Subscriptions pause in the background.** A timer that keeps firing while the user is in
another app wastes battery for a screen nobody sees. When the app leaves the foreground the
framework stops every subscription, and when it returns it starts the ones the app asks for
again. Commands are different: a request that was already running is allowed to finish, and its
message is applied when it arrives.

## A complete example

A screen with a slow "Load" button and a clock. This listing is compiled with the framework's
tests.

```rust
use std::time::Duration;

use krabik_ui_dsl::app::{App, Component};
use krabik_ui_dsl::command::{Command, Subscription};
use krabik_ui_dsl::ui::{Button, Column, Element, ProgressIndicator, Switch, Text};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    /// The user asked for the items.
    Load,
    /// The request finished.
    Loaded(Vec<String>),
    /// The clock was switched on or off.
    Clock(bool),
    /// One second passed while the clock was on.
    Tick,
}

#[derive(Default)]
struct Feed {
    loading: bool,
    items: Vec<String>,
    clock_on: bool,
    seconds: u32,
}

/// Stands in for a network request: slow, and not allowed on the UI thread.
fn fetch_items() -> Vec<String> {
    std::thread::sleep(Duration::from_millis(50));
    vec!["First".to_string(), "Second".to_string()]
}

impl Component for Feed {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Load => {
                // A second tap while a request runs starts nothing.
                if self.loading {
                    return Command::none();
                }
                self.loading = true;
                return Command::perform(|| Msg::Loaded(fetch_items()));
            }
            Msg::Loaded(items) => {
                self.loading = false;
                self.items = items;
            }
            Msg::Clock(on) => self.clock_on = on,
            Msg::Tick => self.seconds += 1,
        }
        Command::none()
    }

    fn view(&self) -> Element<Msg> {
        let mut column = Column::new()
            .padding(24)
            .spacing(12)
            .child(Button::new("Load").on_click(Msg::Load).enabled(!self.loading));
        if self.loading {
            column = column.child(ProgressIndicator::indeterminate());
        }
        for item in &self.items {
            column = column.child(Text::new(item.clone()));
        }
        column
            .child(Switch::new(self.clock_on).on_toggle(Msg::Clock))
            .child(Text::new(format!("{} s", self.seconds)))
            .into()
    }
}

impl App for Feed {
    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        if self.clock_on {
            vec![Subscription::every("clock", Duration::from_secs(1), || {
                Msg::Tick
            })]
        } else {
            Vec::new()
        }
    }
}

// Tests call `update` and look at the state. No thread is started: the
// command is only a description until the framework runs it.
let mut feed = Feed::default();
let command = feed.update(Msg::Load);
assert!(feed.loading);
assert!(!command.is_none());

// A second tap while loading asks for nothing.
assert!(feed.update(Msg::Load).is_none());

// The result is an ordinary message; the test delivers it itself.
feed.update(Msg::Loaded(vec!["A".to_string()]));
assert!(!feed.loading);
assert_eq!(feed.items, ["A"]);

// The clock is state. Whether a timer runs follows from it.
assert!(feed.subscriptions().is_empty());
feed.update(Msg::Clock(true));
assert_eq!(feed.subscriptions().len(), 1);
feed.update(Msg::Tick);
assert_eq!(feed.seconds, 1);
```

The showcase has the same thing as a page you can run: "Async" in the catalog (`async_page` in
`example/rust/src/pages.rs`, `Msg::Load` and `subscriptions` in `example/rust/src/lib.rs`).

## Testing

The listing above shows the pattern, and it is the same three-part pattern as for any other
state change:

1. Send the message that starts the work. Assert on the state (`loading` is true) and, if you
   want, that a command came back (`!command.is_none()`).
2. Send the message the work would produce. Assert on the state after it.
3. For a subscription, assert on what `subscriptions()` returns for a given state.

Step 2 is the reason results are messages. The test does not need the server, the thread or the
two seconds. It only needs to say "and then this came back".

What these unit tests do not cover is that the framework really runs the command and delivers
its result. The framework tests that itself (`the_result_of_a_command_comes_back_as_a_message`
and its neighbours in `rust/src/host/session.rs`), and the showcase's UI tests check it on a
device: `backgroundWorkReportsBackWithoutBlockingTheScreen`, `backgroundWorkSurvivesRecreation`
and `aSubscriptionTicksOnlyWhileItIsWanted` on Android, `testBackgroundWorkAndTheClock` on iOS.

## Things that can go wrong

**A result arrives for a screen the user has left.** The message is still delivered; `update`
decides what to do with it. Usually storing the data is harmless. If the result should be
ignored, record what you are waiting for (a request number, the id of the open note) and compare
in the `Loaded` arm.

**Two requests overlap.** Guard in `update`, as the example does with `if self.loading`, or let
both run and keep the newer result by comparing a request number.

**The work panics.** The thread ends and no message is sent. The app keeps running, but the
state still says "loading". Return a `Result` from the work and put it in the message, so a
failure is a message too: `Msg::Loaded(Result<Vec<Note>, String>)`.

**The activity is recreated while a request runs.** On Android, switching between light and dark
recreates the activity. The application object survives that (chapter 7), and so does its
mailbox: the request finishes, and its message is applied by the new activity. You do not need
to do anything for this.

**A timer is not exact.** `Subscription::every` sleeps for the interval between ticks and each
tick waits for the UI thread to be free. Use it to refresh what is shown, not to measure time.
For a stopwatch, store the start time and compute the elapsed time when a tick arrives.

## Where this lives

- `rust/src/command.rs`: `Command`, `Dispatcher`, `Subscription`, and the mailbox the results
  travel through.
- `rust/src/app.rs`: `Component::update`, `App::subscriptions`.
- `rust/src/host/session.rs`: `apply`, `drain`, `sync_subscriptions`, `set_active`.
- `example/rust/src/lib.rs`, `example/rust/src/pages.rs`: the "Async" page.
- How it works inside: [Commands and the mailbox](../internals/11-commands-and-the-mailbox.md).

Back to the [index](README.md).
