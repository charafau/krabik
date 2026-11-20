//! What a platform has to provide.

use crate::element::RawElement;
use crate::engine::runtime::Runtime;
use crate::engine::tree::NodeId;
use crate::navigation::{Route, ScreenTransition};
use crate::presentation::Dialog;
use crate::scaffold::{Chrome, Snackbar};
use crate::style::Appearance;

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
    fn render(&mut self, cx: &mut P::Cx<'_>, element: &RawElement) -> Result<(), P::Error>;
}

/// The native chrome: the screen stack, toolbar, bottom navigation, system
/// alert and bottom sheet. Rust decides what is shown; the controller only
/// carries it out.
pub(crate) trait Controller<P: Platform> {
    /// Pushes a screen whose content is rendered by host `host`.
    fn push(&self, cx: &mut P::Cx<'_>, host: u64, route: &Route) -> Result<(), P::Error>;
    /// Pops `count` screens in one step with the top screen's transition.
    fn pop(
        &self,
        cx: &mut P::Cx<'_>,
        count: usize,
        transition: ScreenTransition,
    ) -> Result<(), P::Error>;
    /// Shows the main root (no navigation) or the screen stack.
    fn set_root_visible(&self, cx: &mut P::Cx<'_>, visible: bool) -> Result<(), P::Error>;
    /// Applies the top screen's app bar, bottom navigation, floating action
    /// button and drawer. An empty chrome hides all of them. The snackbar is
    /// not part of this call.
    fn set_chrome(&self, cx: &mut P::Cx<'_>, chrome: &Chrome) -> Result<(), P::Error>;
    /// Shows a snackbar, replacing any that is up.
    fn show_snackbar(&self, cx: &mut P::Cx<'_>, snackbar: &Snackbar) -> Result<(), P::Error>;
    /// Removes the snackbar without reporting back.
    fn dismiss_snackbar(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    /// Tells the platform whether the application currently handles the
    /// system back action. When it does not, the platform's own behaviour
    /// (leaving the app, with its predictive animation) applies.
    fn set_back_enabled(&self, cx: &mut P::Cx<'_>, enabled: bool) -> Result<(), P::Error>;
    /// Makes the whole application light or dark, or lets it follow the
    /// device again. The platform reports the colour scheme that results
    /// the way it reports a change on the device.
    fn set_appearance(&self, cx: &mut P::Cx<'_>, appearance: Appearance) -> Result<(), P::Error>;
    /// Shows the system alert.
    fn show_dialog(&self, cx: &mut P::Cx<'_>, dialog: &Dialog) -> Result<(), P::Error>;
    /// Closes the system alert without reporting back.
    fn dismiss_dialog(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    /// Presents the bottom sheet around host `host`.
    fn present_sheet(&self, cx: &mut P::Cx<'_>, host: u64) -> Result<(), P::Error>;
    /// Dismisses the bottom sheet without reporting back.
    fn dismiss_sheet(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    /// The content of the presented sheet now needs `height` logical points.
    /// A platform whose sheet sizes itself to its content ignores this.
    fn sheet_resized(&self, cx: &mut P::Cx<'_>, height: f32) -> Result<(), P::Error>;
}

/// Something the user did to the native chrome of the top screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChromeEvent {
    /// The floating action button was pressed.
    Fab,
    /// A drawer destination was picked.
    DrawerSelected(usize),
    /// The user opened the drawer.
    DrawerOpened,
    /// The user closed the drawer.
    DrawerClosed,
    /// The snackbar's action was pressed, which also removes it.
    SnackbarAction,
    /// The snackbar timed out or was swiped away.
    SnackbarDismissed,
}

impl ChromeEvent {
    /// The number each event travels as, by name. Kotlin, Swift and the C
    /// header repeat these; `the_platforms_use_the_same_event_codes` reads
    /// those files and fails when one of them disagrees.
    pub(crate) const CODES: [(&'static str, i64); 6] = [
        ("Fab", 0),
        ("DrawerSelected", 1),
        ("DrawerOpened", 2),
        ("DrawerClosed", 3),
        ("SnackbarAction", 4),
        ("SnackbarDismissed", 5),
    ];

    /// Decodes the event from the two numbers the platforms report.
    #[cfg_attr(
        not(any(test, target_os = "android", target_os = "ios")),
        allow(dead_code)
    )]
    pub(crate) fn from_code(kind: i64, index: i64) -> Option<Self> {
        let (name, _) = Self::CODES.iter().find(|(_, code)| *code == kind)?;
        Some(match *name {
            "Fab" => Self::Fab,
            "DrawerSelected" => Self::DrawerSelected(usize::try_from(index).ok()?),
            "DrawerOpened" => Self::DrawerOpened,
            "DrawerClosed" => Self::DrawerClosed,
            "SnackbarAction" => Self::SnackbarAction,
            "SnackbarDismissed" => Self::SnackbarDismissed,
            _ => return None,
        })
    }

    /// The name this event has in [`ChromeEvent::CODES`].
    #[cfg(test)]
    fn name(self) -> &'static str {
        match self {
            Self::Fab => "Fab",
            Self::DrawerSelected(_) => "DrawerSelected",
            Self::DrawerOpened => "DrawerOpened",
            Self::DrawerClosed => "DrawerClosed",
            Self::SnackbarAction => "SnackbarAction",
            Self::SnackbarDismissed => "SnackbarDismissed",
        }
    }
}

#[cfg(test)]
mod tests {
    //! The numbers and durations the two shells repeat. They are written in
    //! Kotlin, Swift and C, where the Rust compiler cannot see them, so
    //! these tests read the files.

    use super::*;

    const KOTLIN: &str =
        include_str!("../../../shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt");
    const SWIFT: &str = include_str!("../../../shared/ios/KrabikHostController.swift");
    const HEADER: &str = include_str!("../../../shared/ios/KrabikIOS.h");
    const ANDROID_INTEGERS: &str =
        include_str!("../../../shared/android/src/main/res/values/krabik_integers.xml");

    /// Every `<prefix><Name> ... = <number>` in `source`, with the name
    /// reduced to lower-case letters so `DRAWER_OPENED` and `DrawerOpened`
    /// compare equal.
    fn constants(source: &str, prefix: &str) -> Vec<(String, i64)> {
        let mut found = Vec::new();
        for line in source.lines() {
            let Some(start) = line.find(prefix) else {
                continue;
            };
            let rest = &line[start + prefix.len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .filter(|c| *c != '_')
                .collect::<String>()
                .to_lowercase();
            let Some(value) = rest.split('=').nth(1) else {
                continue;
            };
            let digits: String = value.chars().filter(char::is_ascii_digit).collect();
            if let Ok(value) = digits.parse() {
                found.push((name, value));
            }
        }
        found.sort();
        found
    }

    fn expected() -> Vec<(String, i64)> {
        let mut codes: Vec<(String, i64)> = ChromeEvent::CODES
            .iter()
            .map(|(name, code)| (name.to_lowercase(), *code))
            .collect();
        codes.sort();
        codes
    }

    #[test]
    fn every_event_has_a_code_and_decodes_from_it() {
        let events = [
            ChromeEvent::Fab,
            ChromeEvent::DrawerSelected(3),
            ChromeEvent::DrawerOpened,
            ChromeEvent::DrawerClosed,
            ChromeEvent::SnackbarAction,
            ChromeEvent::SnackbarDismissed,
        ];
        assert_eq!(events.len(), ChromeEvent::CODES.len());
        for event in events {
            let (_, code) = ChromeEvent::CODES
                .iter()
                .find(|(name, _)| *name == event.name())
                .expect("the event is in the table");
            assert_eq!(ChromeEvent::from_code(*code, 3), Some(event));
        }
        assert_eq!(ChromeEvent::from_code(99, 0), None);
        // A negative index is not a destination.
        assert_eq!(ChromeEvent::from_code(1, -1), None);
    }

    #[test]
    fn the_platforms_use_the_same_event_codes() {
        assert_eq!(constants(KOTLIN, "const val EVENT_"), expected(), "Kotlin");
        assert_eq!(constants(SWIFT, "static let event"), expected(), "Swift");
        assert_eq!(
            constants(HEADER, "KrabikChromeEvent"),
            expected(),
            "C header"
        );
    }

    #[test]
    fn a_screen_transition_takes_the_same_time_on_both_platforms() {
        let android = ANDROID_INTEGERS
            .lines()
            .find(|line| line.contains("name=\"krabik_transition_ms\""))
            .and_then(|line| line.split('>').nth(1))
            .and_then(|value| value.split('<').next())
            .and_then(|value| value.trim().parse::<f64>().ok())
            .expect("the Android integer resource");
        let ios = SWIFT
            .lines()
            .find(|line| line.contains("static let transitionSeconds"))
            .and_then(|line| line.split('=').nth(1))
            .and_then(|value| value.trim().parse::<f64>().ok())
            .expect("the Swift constant");
        assert_eq!(android, ios * 1000.0);
    }
}
