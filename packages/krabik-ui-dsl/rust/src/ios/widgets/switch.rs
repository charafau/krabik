//! The `switch` widget on iOS.

use crate::ios::events::dispatch_checked;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{add_action, schedule};
use crate::tree::NodeId;
use crate::widgets::SwitchProps;
use objc2::MainThreadMarker;
use objc2_ui_kit::{UIControlEvents, UISwitch};

pub(crate) fn wire_switch(
    switch: &UISwitch,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        switch,
        UIControlEvents::ValueChanged,
        "krabik.switch",
        mtm,
        move |action| {
            let checked = action
                .sender()
                .and_then(|sender| sender.downcast::<UISwitch>().ok())
                .map(|switch| switch.isOn())
                .unwrap_or(false);
            schedule(move || dispatch_checked(handle, host, id, checked));
        },
    );
}

/// Creates the native view of a switch.
pub(crate) fn create(props: &SwitchProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let switch = UISwitch::new(site.mtm);
    switch.setOn(props.checked);
    wire_switch(&switch, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(switch.clone());
    ViewHandle::new(view, Content::Switch(switch), node)
}
