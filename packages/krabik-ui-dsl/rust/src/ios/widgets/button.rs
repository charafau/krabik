//! The `button` widget on iOS.

use crate::ios::events::dispatch_click;
use crate::ios::{add_action, color, schedule, system_font};
use crate::tree::NodeId;
use crate::ui::ButtonProps;
use objc2::MainThreadMarker;
use objc2_foundation::NSString;
use objc2_ui_kit::{UIButton, UIControlEvents, UIControlState};

pub(crate) fn update_button(button: &UIButton, props: &ButtonProps) {
    let title = NSString::from_str(&props.label);
    button.setTitle_forState(Some(&title), UIControlState::Normal);
    let text_color = color(props.color);
    button.setTitleColor_forState(Some(&text_color), UIControlState::Normal);

    if let Some(label) = button.titleLabel() {
        let font = system_font(props.size, props.weight);
        // SAFETY: the font is retained by the label for as long as it is set.
        unsafe { label.setFont(Some(&font)) };
    }
}

pub(crate) fn wire_click(
    button: &UIButton,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.click",
        mtm,
        move |_| schedule(move || dispatch_click(handle, host, id)),
    );
}
