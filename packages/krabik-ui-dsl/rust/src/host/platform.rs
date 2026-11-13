//! What a platform has to provide.

use crate::element::Element;
use crate::engine::runtime::Runtime;
use crate::engine::tree::NodeId;
use crate::navigation::{Screen, ScreenTransition};
use crate::presentation::Dialog;

use super::Actions;

/// What one native host view renders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HostKind {
    /// The main application root.
    Main,
    /// One screen in the native navigation stack, by screen key.
    Screen(String),
    /// The content of the presented bottom sheet.
    Sheet,
}

/// The types a platform plugs into a [`Session`](super::Session).
pub(crate) trait Platform: Sized {
    /// Whatever a native call needs for the duration of one entry point: the
    /// JNI environment on Android, the main-thread marker on iOS.
    type Cx<'a>;
    /// The error a native call can fail with.
    type Error;
    /// One native root view and its rendering state.
    type Host: Host<Self>;
    /// The native object that owns navigation, the toolbar and modals.
    type Controller: Controller<Self>;
}

/// One native root view that Rust renders into.
pub(crate) trait Host<P: Platform> {
    /// What this host shows.
    fn kind(&self) -> &HostKind;
    /// The engine state of this host.
    fn runtime(&self) -> &Runtime;
    /// The engine state of this host.
    fn runtime_mut(&mut self) -> &mut Runtime;
    /// The actions of a widget inside a recycled row. Such widgets are not
    /// part of the runtime's tree, so the host records them when it binds
    /// the row.
    fn row_actions(&self, id: NodeId) -> Option<&Actions>;
    /// The list and item index a recycled-row widget belongs to.
    fn row_of(&self, id: NodeId) -> Option<(NodeId, usize)>;
    /// Reconciles the host with a new description and applies the changes to
    /// the native views.
    fn render(&mut self, cx: &mut P::Cx<'_>, element: &Element) -> Result<(), P::Error>;
}

/// The native chrome: the screen stack, toolbar, bottom navigation, system
/// alert and bottom sheet. Rust decides what is shown; the controller only
/// carries it out.
pub(crate) trait Controller<P: Platform> {
    /// Pushes a screen whose content is rendered by host `host`.
    fn push(&self, cx: &mut P::Cx<'_>, host: u64, screen: &Screen) -> Result<(), P::Error>;
    /// Pops `count` screens in one step with the top screen's transition.
    fn pop(
        &self,
        cx: &mut P::Cx<'_>,
        count: usize,
        transition: ScreenTransition,
    ) -> Result<(), P::Error>;
    /// Shows the main root (no navigation) or the screen stack.
    fn set_root_visible(&self, cx: &mut P::Cx<'_>, visible: bool) -> Result<(), P::Error>;
    /// Applies the top screen's toolbar and bottom navigation; `None` hides both.
    fn set_chrome(&self, cx: &mut P::Cx<'_>, screen: Option<&Screen>) -> Result<(), P::Error>;
    /// Shows the system alert.
    fn show_dialog(&self, cx: &mut P::Cx<'_>, dialog: &Dialog) -> Result<(), P::Error>;
    /// Closes the system alert without reporting back.
    fn dismiss_dialog(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    /// Presents the bottom sheet around host `host`.
    fn present_sheet(&self, cx: &mut P::Cx<'_>, host: u64) -> Result<(), P::Error>;
    /// Dismisses the bottom sheet without reporting back.
    fn dismiss_sheet(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
}
