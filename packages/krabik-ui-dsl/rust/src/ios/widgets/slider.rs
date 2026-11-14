//! The `slider` widget on iOS.

use crate::ios::events::dispatch_value;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{add_action, color, schedule};
use crate::tree::NodeId;
use crate::ui::{SliderProps, snap_slider_value};
use objc2::MainThreadMarker;
use objc2_ui_kit::{UIControlEvents, UISlider};

pub(crate) fn update_slider(slider: &UISlider, props: &SliderProps) {
    let max = props.max.max(props.min + 0.001);
    slider.setMinimumValue(props.min);
    slider.setMaximumValue(max);
    let value = snap_slider_value(props.value, props.min, max, props.step);
    if (slider.value() - value).abs() > f32::EPSILON {
        slider.setValue(value);
    }
    let tint = color(props.color);
    slider.setMinimumTrackTintColor(Some(&tint));
}

pub(crate) fn wire_slider(
    slider: &UISlider,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        slider,
        UIControlEvents::ValueChanged,
        "krabik.slider",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UISlider>().ok())
                .map(|slider| slider.value())
                .unwrap_or(0.0);
            schedule(move || dispatch_value(handle, host, id, f64::from(value)));
        },
    );
}

/// Creates the native view of a slider.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let slider = UISlider::new(site.mtm);
    wire_slider(&slider, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(slider.clone());
    ViewHandle::new(view, Content::Slider(slider), node)
}
