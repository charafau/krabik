//! The `switch` widget on iOS.

use crate::ios::events::dispatch_checked;
use crate::ios::{add_action, schedule};
use crate::tree::NodeId;
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
