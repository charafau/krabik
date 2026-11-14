//! The iOS pieces of a session: the host and the Swift `KrabikHostController`.

use crate::host::{Actions, Controller, Host, HostKind, Platform, Session};
use crate::ios::host::IosState;
use crate::navigation::{Route, ScreenTransition};
use crate::presentation::Dialog;
use crate::runtime::Runtime;
use crate::scaffold::{Chrome, Snackbar};
use crate::tree::NodeId;
use crate::ui::RawElement;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2_foundation::{NSArray, NSNumber, NSString};
use std::convert::Infallible;

/// The iOS pieces of a [`Session`].
pub(crate) struct Ios;

impl Platform for Ios {
    type Cx<'a> = MainThreadMarker;
    type Error = Infallible;
    type Host = IosState;
    type Controller = IosController;
}

pub(crate) type IosSession = Session<Ios>;

// SAFETY: every host and the controller are only touched on the main thread,
// serialized by the global session mutex.
unsafe impl Send for Session<Ios> {}

impl Host<Ios> for IosState {
    fn kind(&self) -> &HostKind {
        &self.kind
    }

    fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    fn row_actions(&self, id: NodeId) -> Option<&Actions> {
        self.row_actions.get(&id)
    }

    fn row_of(&self, id: NodeId) -> Option<(NodeId, usize)> {
        self.row_nodes.get(&id).copied()
    }

    fn render(
        &mut self,
        _mtm: &mut MainThreadMarker,
        element: &RawElement,
    ) -> Result<(), Infallible> {
        IosState::render(self, element);
        Ok(())
    }
}

/// The Swift `KrabikHostController`, which owns the navigation stack and
/// bar, the tab bar, the system alert and the sheet.
pub(crate) struct IosController(pub(crate) Retained<NSObject>);

impl Controller<Ios> for IosController {
    fn push(&self, _: &mut MainThreadMarker, host: u64, route: &Route) -> Result<(), Infallible> {
        controller_push(&self.0, host, route.transition.code());
        Ok(())
    }

    fn pop(
        &self,
        _: &mut MainThreadMarker,
        count: usize,
        transition: ScreenTransition,
    ) -> Result<(), Infallible> {
        controller_pop(&self.0, count, transition.code());
        Ok(())
    }

    fn set_root_visible(&self, _: &mut MainThreadMarker, visible: bool) -> Result<(), Infallible> {
        controller_set_root_visible(&self.0, visible);
        Ok(())
    }

    fn set_chrome(&self, _: &mut MainThreadMarker, chrome: &Chrome) -> Result<(), Infallible> {
        // A screen without an app bar hides the navigation bar.
        let bar = chrome.app_bar.as_ref();
        controller_set_toolbar(
            &self.0,
            bar.map_or("", |bar| bar.title.as_str()),
            bar.and_then(|bar| bar.subtitle.as_deref()).unwrap_or(""),
            bar.and_then(|bar| bar.back_title.as_deref())
                .unwrap_or("Back"),
            bar.is_none(),
            bar.map_or(&[][..], |bar| bar.actions.as_slice()),
        );
        controller_set_bottom_navigation(&self.0, chrome.bottom_navigation.as_ref());

        let fab = chrome.floating_action_button.as_ref();
        let label = NSString::from_str(fab.map_or("", |fab| fab.label.as_str()));
        let icon = NSString::from_str(fab.and_then(|fab| fab.icon.ios_name()).unwrap_or(""));
        // SAFETY: see `controller_push`.
        let _: () = unsafe {
            objc2::msg_send![
                &*self.0,
                krabikSetFab: &*label,
                icon: &*icon,
                visible: fab.is_some()
            ]
        };

        let drawer = chrome.drawer.as_ref();
        let items = drawer.map_or(&[][..], |drawer| drawer.items.as_slice());
        let labels = string_array(items.iter().map(|item| item.label.as_str()));
        let icons = string_array(items.iter().map(|item| item.icon.ios_name().unwrap_or("")));
        // -1 highlights nothing.
        let selected = drawer
            .and_then(|drawer| drawer.selected)
            .map_or(-1, |index| index as i64);
        // SAFETY: see `controller_push`.
        let _: () = unsafe {
            objc2::msg_send![
                &*self.0,
                krabikSetDrawer: &*labels,
                icons: &*icons,
                selected: selected,
                present: drawer.is_some(),
                open: drawer.is_some_and(|drawer| drawer.open)
            ]
        };
        Ok(())
    }

    fn show_snackbar(
        &self,
        _: &mut MainThreadMarker,
        snackbar: &Snackbar,
    ) -> Result<(), Infallible> {
        let message = NSString::from_str(&snackbar.message);
        // An empty label means no action button.
        let action = NSString::from_str(snackbar.action.as_deref().unwrap_or(""));
        // SAFETY: see `controller_push`.
        let _: () =
            unsafe { objc2::msg_send![&*self.0, krabikShowSnackbar: &*message, action: &*action] };
        Ok(())
    }

    fn dismiss_snackbar(&self, _: &mut MainThreadMarker) -> Result<(), Infallible> {
        // SAFETY: see `controller_push`.
        let _: () = unsafe { objc2::msg_send![&*self.0, krabikDismissSnackbar] };
        Ok(())
    }

    fn set_back_enabled(&self, _: &mut MainThreadMarker, _enabled: bool) -> Result<(), Infallible> {
        // iOS has no system back action; the navigation bar's back button
        // asks the session directly.
        Ok(())
    }

    fn show_dialog(&self, _: &mut MainThreadMarker, dialog: &Dialog) -> Result<(), Infallible> {
        controller_show_dialog(&self.0, dialog);
        Ok(())
    }

    fn dismiss_dialog(&self, _: &mut MainThreadMarker) -> Result<(), Infallible> {
        controller_dismiss_dialog(&self.0);
        Ok(())
    }

    fn present_sheet(&self, _: &mut MainThreadMarker, host: u64) -> Result<(), Infallible> {
        controller_present_sheet(&self.0, host);
        Ok(())
    }

    fn dismiss_sheet(&self, _: &mut MainThreadMarker) -> Result<(), Infallible> {
        controller_dismiss_sheet(&self.0);
        Ok(())
    }
}

pub(crate) fn controller_push(controller: &NSObject, host: u64, transition: i32) {
    // SAFETY: the controller is a live Swift object with this @objc method.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikPush: host as i64,
            transition: transition as i64
        ]
    };
}

pub(crate) fn controller_set_toolbar(
    controller: &NSObject,
    title: &str,
    subtitle: &str,
    back_title: &str,
    hidden: bool,
    actions: &[crate::scaffold::AppBarAction],
) {
    let title = NSString::from_str(title);
    let subtitle = NSString::from_str(subtitle);
    let back_title = NSString::from_str(back_title);
    let labels: Vec<Retained<NSString>> = actions
        .iter()
        .map(|action| NSString::from_str(&action.label))
        .collect();
    let icons: Vec<Retained<NSString>> = actions
        .iter()
        .map(|action| NSString::from_str(action.icon.ios_name().unwrap_or("")))
        .collect();
    let labels = NSArray::from_retained_slice(&labels);
    let icons = NSArray::from_retained_slice(&icons);
    // SAFETY: the controller is a live Swift object with this @objc method.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikSetToolbar: &*title,
            subtitle: &*subtitle,
            backTitle: &*back_title,
            hidden: hidden,
            actionLabels: &*labels,
            actionIcons: &*icons
        ]
    };
}

pub(crate) fn controller_pop(controller: &NSObject, count: usize, transition: i32) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe {
        objc2::msg_send![controller, krabikPop: count as i64, transition: transition as i64]
    };
}

pub(crate) fn controller_set_root_visible(controller: &NSObject, visible: bool) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikSetRootVisible: visible] };
}

pub(crate) fn string_array<'a>(
    values: impl Iterator<Item = &'a str>,
) -> Retained<NSArray<NSString>> {
    let values: Vec<Retained<NSString>> = values.map(NSString::from_str).collect();
    NSArray::from_retained_slice(&values)
}

/// Applies the bottom navigation bar; `None` hides it.
pub(crate) fn controller_set_bottom_navigation(
    controller: &NSObject,
    bar: Option<&crate::navigation::BottomNavigation>,
) {
    let items = bar.map_or(&[][..], |bar| bar.items.as_slice());
    let labels = string_array(items.iter().map(|item| item.label.as_str()));
    let icons = string_array(items.iter().map(|item| item.icon.ios_name().unwrap_or("")));
    let selected = bar.map_or(0, |bar| bar.selected) as i64;
    // SAFETY: see `controller_push`.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikSetBottomNavigation: &*labels,
            icons: &*icons,
            selected: selected
        ]
    };
}

pub(crate) fn controller_show_dialog(controller: &NSObject, dialog: &crate::presentation::Dialog) {
    let title = NSString::from_str(&dialog.title);
    let message = NSString::from_str(dialog.message.as_deref().unwrap_or(""));
    let labels = string_array(dialog.buttons.iter().map(|b| b.label.as_str()));
    let roles: Vec<Retained<NSNumber>> = dialog
        .buttons
        .iter()
        .map(|b| NSNumber::new_i32(b.role.code()))
        .collect();
    let roles = NSArray::from_retained_slice(&roles);
    // SAFETY: see `controller_push`.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikShowDialog: &*title,
            message: &*message,
            labels: &*labels,
            roles: &*roles
        ]
    };
}

pub(crate) fn controller_dismiss_dialog(controller: &NSObject) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikDismissDialog] };
}

pub(crate) fn controller_present_sheet(controller: &NSObject, host: u64) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikPresentSheet: host as i64] };
}

pub(crate) fn controller_dismiss_sheet(controller: &NSObject) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikDismissSheet] };
}
