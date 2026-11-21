//! The `range_slider` widget on iOS.
//!
//! UIKit has no slider with two thumbs, so the control is
//! `KrabikRangeSlider`, a Swift class of the shared host sources. It is
//! found by name and driven through its Objective-C methods.

use crate::host::NodeEvent;
use crate::ios::events::dispatch;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, add_action, cast_view, color, schedule};
use crate::ui::RangeSliderProps;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, NSObjectProtocol};
use objc2_ui_kit::{UIControl, UIControlEvents};

pub(crate) fn update_range_slider(slider: &UIControl, props: &RangeSliderProps) {
    // A plain control stands in when the class is missing; see `create`.
    if !slider.respondsToSelector(objc2::sel!(setLow:high:)) {
        return;
    }
    let tint = color(props.color);
    let (low, high) = props.fractions();
    // SAFETY: the control is a `KrabikRangeSlider`, which has these methods
    // with these argument types.
    unsafe {
        let _: () = objc2::msg_send![slider, configureWithColor: &*tint];
        let _: () = objc2::msg_send![slider, setLow: low, high: high];
    }
}

/// Creates the native view of a range slider.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let slider: Retained<UIControl> = match AnyClass::get(c"KrabikRangeSlider") {
        // SAFETY: the class is a `UIControl` subclass with a plain `init`.
        Some(class) => unsafe { objc2::msg_send![class, new] },
        None => {
            crate::log::error(
                "KrabikRangeSlider is not in the app; add shared/ios/KrabikRangeSlider.swift",
            );
            UIControl::new(site.mtm)
        }
    };
    let (handle, host, id) = (site.handle, site.host, site.node);
    add_action(
        &slider,
        UIControlEvents::ValueChanged,
        "krabik.range",
        site.mtm,
        move |action| {
            let Some(sender) = action.sender() else {
                return;
            };
            // SAFETY: the sender is the `KrabikRangeSlider` this action was
            // added to, whose `low` and `high` are `Float` properties.
            let (low, high): (f32, f32) = unsafe {
                (
                    objc2::msg_send![&*sender, low],
                    objc2::msg_send![&*sender, high],
                )
            };
            schedule(move || dispatch(handle, host, id, NodeEvent::Range(low, high)));
        },
    );
    let view = cast_view(slider.clone());
    ViewHandle::new(view, Content::RangeSlider(slider), node)
}
