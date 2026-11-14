//! The `radio_group` widget on iOS.

use crate::ios::events::dispatch_selected;
use crate::ios::widgets::check::update_check;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{add_action, schedule};
use crate::tree::NodeId;
use crate::ui::{CheckProps, RadioGroupProps};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_ui_kit::{
    NSObjectUIAccessibility, UILayoutConstraintAxis, UIStackViewAlignment, UIStackViewDistribution,
};
use objc2_ui_kit::{UIButton, UIButtonType, UIControlEvents, UIStackView};

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_radio_group(
    stack: &UIStackView,
    buttons: &mut Vec<Retained<UIButton>>,
    labels: &mut Vec<String>,
    props: &RadioGroupProps,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    if *labels != props.options {
        for button in buttons.iter() {
            button.removeFromSuperview();
        }
        buttons.clear();
        labels.clear();
        for (index, label) in props.options.iter().enumerate() {
            let button = UIButton::buttonWithType(UIButtonType::Custom, mtm);
            wire_radio_group_item(&button, handle, host, id, index, mtm);
            buttons.push(button);
            labels.push(label.clone());
        }
        for button in buttons.iter() {
            stack.addArrangedSubview(button);
        }
    }

    stack.setSpacing(f64::from(props.spacing));
    for (index, (button, label)) in buttons.iter().zip(props.options.iter()).enumerate() {
        update_check(
            button,
            &CheckProps {
                label: label.clone(),
                checked: props.selected == Some(index),
                on_toggle: crate::ui::Handler::none(),
                size: props.size,
                color: props.color,
                weight: props.weight,
            },
            true,
        );
        let accessibility = NSString::from_str(label);
        button.setAccessibilityLabel(Some(&accessibility), mtm);
    }
}

pub(crate) fn wire_radio_group_item(
    button: &UIButton,
    handle: u64,
    host: u64,
    id: NodeId,
    index: usize,
    mtm: MainThreadMarker,
) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.radio-group",
        mtm,
        move |_| schedule(move || dispatch_selected(handle, host, id, index)),
    );
}

/// Creates the native view of a radio group.
pub(crate) fn create(props: &RadioGroupProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let stack = UIStackView::new(site.mtm);
    stack.setAxis(UILayoutConstraintAxis::Vertical);
    stack.setAlignment(UIStackViewAlignment::Fill);
    stack.setDistribution(UIStackViewDistribution::FillEqually);
    let mut buttons = Vec::new();
    let mut labels = Vec::new();
    update_radio_group(
        &stack,
        &mut buttons,
        &mut labels,
        props,
        site.handle,
        site.host,
        site.node,
        site.mtm,
    );
    let view = cast_view(stack.clone());
    ViewHandle::new(
        view,
        Content::RadioGroup {
            stack,
            buttons,
            labels,
        },
        node,
    )
}
