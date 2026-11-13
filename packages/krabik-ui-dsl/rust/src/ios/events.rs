//! UIKit events on widgets, handed to the session on the next main-queue turn.

use crate::host::NodeEvent;
use crate::ios::{guard, infallible, with_session, with_state};
use crate::tree::NodeId;
use crate::ui::DatePickerValue;
use objc2::MainThreadMarker;

/// Hands an event on one widget to the session.
pub(crate) fn dispatch(handle: u64, host: u64, id: NodeId, event: NodeEvent) {
    // SAFETY: events are delivered on the main queue.
    let mut mtm = unsafe { MainThreadMarker::new_unchecked() };
    guard("event", || {
        with_session(handle, |session| {
            infallible(session.node_event(&mut mtm, host, id, event));
        });
    });
}

pub(crate) fn dispatch_click(handle: u64, host: u64, id: NodeId) {
    dispatch(handle, host, id, NodeEvent::Click);
}

pub(crate) fn dispatch_text(handle: u64, host: u64, id: NodeId, value: &str) {
    dispatch(handle, host, id, NodeEvent::Text(value.to_owned()));
}

pub(crate) fn dispatch_submitted(handle: u64, host: u64, id: NodeId) {
    dispatch(handle, host, id, NodeEvent::Submit);
}

pub(crate) fn dispatch_focus(handle: u64, host: u64, id: NodeId, focused: bool) {
    // A field that lost focus because its cell left the hierarchy keeps its
    // logical focus, so it can be restored when the row comes back.
    let attached = with_state(handle, host, |state| state.field_attached(id)).unwrap_or(true);
    dispatch(handle, host, id, NodeEvent::Focus { focused, attached });
}

pub(crate) fn dispatch_checked(handle: u64, host: u64, id: NodeId, checked: bool) {
    dispatch(handle, host, id, NodeEvent::Toggle(checked));
}

pub(crate) fn dispatch_value(handle: u64, host: u64, id: NodeId, value: f64) {
    dispatch(handle, host, id, NodeEvent::Slider(value as f32));
}

pub(crate) fn dispatch_picker(handle: u64, host: u64, id: NodeId, value: DatePickerValue) {
    dispatch(handle, host, id, NodeEvent::Picker(value));
}

pub(crate) fn dispatch_selected(handle: u64, host: u64, id: NodeId, index: usize) {
    dispatch(handle, host, id, NodeEvent::Select(index));
}

pub(crate) fn dispatch_scroll(handle: u64, host: u64, id: NodeId, offset: f64) {
    dispatch(handle, host, id, NodeEvent::Scroll(offset as f32));
}
