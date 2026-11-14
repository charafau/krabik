//! The `check` widget on iOS.

use crate::ios::events::dispatch_checked;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{add_action, color, schedule, system_font};
use crate::tree::NodeId;
use crate::ui::CheckProps;
use objc2::MainThreadMarker;
use objc2_foundation::NSString;
use objc2_ui_kit::UIButtonType;
use objc2_ui_kit::{
    UIButton, UIControlContentHorizontalAlignment, UIControlEvents, UIControlState, UIImage,
};

pub(crate) fn update_check(button: &UIButton, props: &CheckProps, radio: bool) {
    // The leading space separates the symbol from the label; the builder sets
    // the clean accessibility label.
    let title = NSString::from_str(&format!(" {}", props.label));
    button.setTitle_forState(Some(&title), UIControlState::Normal);
    let text_color = color(props.color);
    button.setTitleColor_forState(Some(&text_color), UIControlState::Normal);
    button.setTitleColor_forState(Some(&text_color), UIControlState::Selected);
    if let Some(label) = button.titleLabel() {
        let font = system_font(props.size, props.weight);
        // SAFETY: the font is retained by the label for as long as it is set.
        unsafe { label.setFont(Some(&font)) };
    }

    let (unchecked, checked) = if radio {
        ("circle", "smallcircle.filled.circle")
    } else {
        ("square", "checkmark.square.fill")
    };
    let symbol = if props.checked { checked } else { unchecked };
    if let Some(image) = UIImage::systemImageNamed(&NSString::from_str(symbol)) {
        button.setImage_forState(Some(&image), UIControlState::Normal);
    }
    button.setImage_forState(None, UIControlState::Selected);
    button.setSelected(props.checked);
    button.setContentHorizontalAlignment(UIControlContentHorizontalAlignment::Left);
    // SAFETY: the color is retained by the view for as long as it is set.
    unsafe { button.setTintColor(Some(&text_color)) };
}

pub(crate) fn wire_check(
    button: &UIButton,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.check",
        mtm,
        move |action| {
            let checked = action
                .sender()
                .and_then(|sender| sender.downcast::<UIButton>().ok())
                .map(|button| !button.isSelected())
                .unwrap_or(false);
            schedule(move || dispatch_checked(handle, host, id, checked));
        },
    );
}

/// Creates the native view of a checkbox, or of a radio button.
pub(crate) fn create(node: &RenderNode, site: &Site, radio: bool) -> ViewHandle {
    let button = UIButton::buttonWithType(UIButtonType::Custom, site.mtm);
    wire_check(&button, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(button.clone());
    ViewHandle::new(view, Content::Check { button, radio }, node)
}
