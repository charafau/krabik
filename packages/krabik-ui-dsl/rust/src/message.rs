//! Typed messages, erased.
//!
//! An application describes what should happen on an event as a value of its
//! own message type: `Button::new("+1").on_click(Msg::Increment)`. The
//! engine and the platforms do not know that type, so each handler is stored
//! as a function that produces a boxed message, and the application's
//! `update` receives it back with its real type.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

/// An application message whose concrete type only the application knows.
pub(crate) type Message = Box<dyn Any>;

/// What to tell the application when an event with payload `A` happens.
///
/// An unset handler means the widget does not report that event.
pub struct Handler<A>(Option<Arc<dyn Fn(A) -> Message + Send + Sync>>);

impl<A: 'static> Handler<A> {
    /// A handler that reports nothing.
    pub const fn none() -> Self {
        Self(None)
    }

    /// A handler that turns the event's payload into a message.
    pub(crate) fn new<M: 'static>(make: impl Fn(A) -> M + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(move |payload| Box::new(make(payload)))))
    }

    /// Whether the widget reports this event.
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// The message for an event, if the widget reports it.
    pub(crate) fn message(&self, payload: A) -> Option<Message> {
        self.0.as_ref().map(|make| make(payload))
    }

    /// Passes every message this handler produces through `wrap`. Used to
    /// lift a child component's messages into its parent's message type.
    pub(crate) fn map(&mut self, wrap: &Wrap) {
        if let Some(make) = self.0.take() {
            let wrap = wrap.clone();
            self.0 = Some(Arc::new(move |payload| wrap(make(payload))));
        }
    }
}

/// A function that re-wraps a message; see [`Handler::map`].
pub(crate) type Wrap = Arc<dyn Fn(Message) -> Message + Send + Sync>;

impl<A> Clone for Handler<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<A> Default for Handler<A> {
    fn default() -> Self {
        Self(None)
    }
}

/// Two handlers are equal when both are set or both are unset.
///
/// Closures cannot be compared, and they do not need to be: the engine reads
/// handlers from the latest tree when an event arrives, so a changed closure
/// never has to be pushed to a native view.
impl<A> PartialEq for Handler<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.is_some() == other.0.is_some()
    }
}

impl<A> Eq for Handler<A> {}

impl<A> fmt::Debug for Handler<A> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(if self.0.is_some() {
            "Handler(set)"
        } else {
            "Handler(none)"
        })
    }
}
