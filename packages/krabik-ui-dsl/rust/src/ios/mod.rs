//! iOS renderer driven directly through UIKit.
//!
//! The Swift side is a thin `UIView`: it forwards its bounds to Rust and hosts
//! the native widgets. Every widget, every property update, every frame change,
//! and every event is handled here on native `UILabel`/`UIButton`/… instances.

use crate::ios::controller::IosSession;
use crate::ios::entry::states;
use crate::ios::host::{IosState, ViewHandle};
use crate::tree::NodeId;
use crate::ui::{EdgeInsets, FontWeight};
use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, Message};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGColor;
use objc2_foundation::NSString;
use objc2_ui_kit::{UIAction, UIColor, UIControl, UIControlEvents, UIFont, UIView};
use std::collections::HashMap;
use std::convert::Infallible;
use std::ptr::NonNull;

mod controller;
mod entry;
mod events;
mod host;
mod images;
mod measure;
mod motion;
mod rows;
mod style;
mod widgets;

pub(crate) fn system_font(size: f32, weight: FontWeight) -> Retained<UIFont> {
    match weight {
        FontWeight::Bold => UIFont::boldSystemFontOfSize(f64::from(size)),
        FontWeight::Normal => UIFont::systemFontOfSize(f64::from(size)),
    }
}

pub(crate) fn color(argb: u32) -> Retained<UIColor> {
    let alpha = f64::from((argb >> 24) & 0xFF) / 255.0;
    let red = f64::from((argb >> 16) & 0xFF) / 255.0;
    let green = f64::from((argb >> 8) & 0xFF) / 255.0;
    let blue = f64::from(argb & 0xFF) / 255.0;
    UIColor::colorWithRed_green_blue_alpha(red, green, blue, alpha)
}

pub(crate) fn cg_color(argb: u32) -> Retained<CGColor> {
    // SAFETY: the color is a valid `UIColor` and `CGColor` is retained.
    unsafe { color(argb).CGColor() }
}

/// The view that hosts the children of the given native parent.
pub(crate) fn parent_host_in(
    views: &HashMap<NodeId, ViewHandle>,
    root: &Retained<UIView>,
    parent: Option<NodeId>,
) -> Retained<UIView> {
    match parent.and_then(|id| views.get(&id)) {
        Some(handle) => handle.children_host(),
        None => root.clone(),
    }
}

/// Removes every subview, used before recreating a reused cell's row.
pub(crate) fn remove_subviews(view: &UIView) {
    for subview in view.subviews().iter() {
        subview.removeFromSuperview();
    }
}

pub(crate) fn cast_view<T: Message>(view: Retained<T>) -> Retained<UIView> {
    // SAFETY: every caller passes a `UIView` subclass.
    unsafe { Retained::cast_unchecked(view) }
}

pub(crate) fn inset(size: CGSize, padding: EdgeInsets) -> CGRect {
    let left = f64::from(padding.left);
    let top = f64::from(padding.top);
    let right = f64::from(padding.right);
    let bottom = f64::from(padding.bottom);
    CGRect::new(
        CGPoint::new(left, top),
        CGSize::new(
            (size.width - left - right).max(0.0),
            (size.height - top - bottom).max(0.0),
        ),
    )
}

pub(crate) fn as_any(view: &UIView) -> &AnyObject {
    // SAFETY: every Objective-C object can be viewed as `AnyObject`.
    unsafe { &*(view as *const UIView as *const AnyObject) }
}

pub(crate) fn add_action(
    control: &UIControl,
    events: UIControlEvents,
    identifier: &str,
    mtm: MainThreadMarker,
    handler: impl Fn(&UIAction) + 'static,
) {
    let block: RcBlock<dyn Fn(NonNull<UIAction>)> =
        RcBlock::new(move |action: NonNull<UIAction>| {
            // SAFETY: the block is invoked with a live action by UIKit.
            handler(unsafe { action.as_ref() });
        });
    let title = NSString::from_str("");
    let identifier = NSString::from_str(identifier);
    // SAFETY: the block is a valid `UIActionHandler` and is retained by the
    // action, which is retained by the control.
    let action = unsafe {
        UIAction::actionWithTitle_image_identifier_handler(
            &title,
            None,
            Some(&identifier),
            RcBlock::as_ptr(&block),
            mtm,
        )
    };
    control.addAction_forControlEvents(&action, events);
}

/// Defers an event to the next main-queue turn.
///
/// UIKit can deliver another action synchronously while we are rebuilding the
/// tree (for example `EditingDidEnd` when a focused field is removed). Running
/// the handler asynchronously keeps the state lock from being re-entered.
pub(crate) fn schedule(work: impl FnOnce() + Send + 'static) {
    DispatchQueue::main().exec_async(work);
}

pub(crate) fn with_state<T>(
    handle: u64,
    host: u64,
    body: impl FnOnce(&mut IosState) -> T,
) -> Option<T> {
    let mut states = states().lock().expect("ios states");
    states
        .get_mut(&handle)
        .and_then(|session| session.host_mut(host))
        .map(body)
}

pub(crate) fn with_session<T>(handle: u64, body: impl FnOnce(&mut IosSession) -> T) -> Option<T> {
    let mut states = states().lock().expect("ios states");
    states.get_mut(&handle).map(body)
}

/// Runs an event handler, reporting an Objective-C exception instead of
/// letting it unwind through the C ABI boundary and abort the process.
pub(crate) fn guard(label: &str, body: impl FnOnce()) {
    let result = objc2::exception::catch(std::panic::AssertUnwindSafe(body));
    if let Err(exception) = result {
        let message = match exception {
            Some(exception) => format!("{exception:?}"),
            None => "nil exception".to_string(),
        };
        eprintln!("krabik ios: {label} handler raised an exception: {message}");
    }
}

/// Unwraps a result that cannot fail.
pub(crate) fn infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}
