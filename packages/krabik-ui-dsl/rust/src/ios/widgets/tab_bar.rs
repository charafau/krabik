//! The `tab_bar` widget on iOS.

use crate::ios::events::dispatch_selected;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{add_action, color, schedule};
use crate::tree::NodeId;
use crate::ui::TabBarProps;
use objc2::MainThreadMarker;
use objc2_foundation::NSString;
use objc2_ui_kit::{UIControlEvents, UISegmentedControl};

pub(crate) fn update_tab_bar(control: &UISegmentedControl, props: &TabBarProps) {
    let current: Vec<String> = (0..control.numberOfSegments())
        .filter_map(|index| {
            control
                .titleForSegmentAtIndex(index)
                .map(|title| title.to_string())
        })
        .collect();
    if current != props.tabs {
        control.removeAllSegments();
        for (index, tab) in props.tabs.iter().enumerate() {
            let title = NSString::from_str(tab);
            control.insertSegmentWithTitle_atIndex_animated(Some(&title), index, false);
        }
    }
    let selected = if props.tabs.is_empty() {
        -1
    } else {
        props.selected.min(props.tabs.len() - 1) as isize
    };
    if control.selectedSegmentIndex() != selected {
        control.setSelectedSegmentIndex(selected);
    }
    // SAFETY: the color is a valid `UIColor` for the lifetime of the call.
    unsafe { control.setTintColor(Some(&color(props.color))) };
}

pub(crate) fn wire_tab_bar(
    control: &UISegmentedControl,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        control,
        UIControlEvents::ValueChanged,
        "krabik.tab-bar",
        mtm,
        move |action| {
            let index = action
                .sender()
                .and_then(|sender| sender.downcast::<UISegmentedControl>().ok())
                .map(|control| control.selectedSegmentIndex());
            if let Some(index) = index
                && index >= 0
            {
                schedule(move || dispatch_selected(handle, host, id, index as usize));
            }
        },
    );
}

/// Creates the native view of a tab bar.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let control = UISegmentedControl::new(site.mtm);
    wire_tab_bar(&control, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(control.clone());
    ViewHandle::new(view, Content::TabBar(control), node)
}
