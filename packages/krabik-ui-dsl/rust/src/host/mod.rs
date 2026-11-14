//! The platform-independent half of a running application.
//!
//! A [`Session`] owns the application, one [`Host`] per native root view
//! (the main view, each navigation screen, the sheet) and mirrors the
//! application's navigation, chrome, dialog and sheet onto the platform's
//! [`Controller`]. Native events enter through [`Session::node_event`] and
//! its siblings and are resolved by [`events`].
//!
//! Android and iOS only provide the [`Platform`] pieces: how to render into a
//! native view and how to drive the native chrome.

pub(crate) mod events;
mod platform;
mod session;

pub(crate) use events::{Actions, NodeEvent};
pub(crate) use platform::{ChromeEvent, Controller, Host, HostKind, Platform};
// Only the platform renderers create sessions.
#[cfg_attr(
    not(any(target_os = "android", target_os = "ios")),
    allow(unused_imports)
)]
pub(crate) use session::Session;
