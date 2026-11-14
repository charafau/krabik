//! The `text` widget on iOS.

use crate::ios::ViewHandle;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{color, system_font};
use crate::ui::{TextAlign, TextProps};
use objc2_foundation::NSString;
use objc2_ui_kit::UIView;
use objc2_ui_kit::{NSLineBreakMode, NSTextAlignment, UILabel};

pub(crate) fn update_label(label: &UILabel, props: &TextProps) {
    label.setText(Some(&NSString::from_str(&props.content)));
    let font = system_font(props.size, props.weight);
    // SAFETY: the font is retained by the label for as long as it is set.
    unsafe { label.setFont(Some(&font)) };
    let text_color = color(props.color);
    // SAFETY: the color is retained by the label for as long as it is set.
    unsafe { label.setTextColor(Some(&text_color)) };
    label.setNumberOfLines(props.max_lines.map_or(0, |lines| lines.max(1) as isize));
    label.setLineBreakMode(if props.ellipsis {
        NSLineBreakMode::ByTruncatingTail
    } else {
        NSLineBreakMode::ByWordWrapping
    });
    label.setTextAlignment(match props.align {
        TextAlign::Start => NSTextAlignment(0),
        TextAlign::Center => NSTextAlignment(1),
        TextAlign::End => NSTextAlignment(2),
    });
}

/// Creates the native view of a text.
pub(crate) fn create(props: &TextProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let wrapper = UIView::new(site.mtm);
    let label = UILabel::new(site.mtm);
    update_label(&label, props);
    wrapper.addSubview(&label);
    ViewHandle::new(wrapper, Content::Label(label), node)
}
