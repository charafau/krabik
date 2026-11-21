//! The `text` widget on iOS.

use crate::ios::ViewHandle;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{color, font_of};
use crate::ui::{TextAlign, TextProps};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{
    NSAttributedString, NSAttributedStringKey, NSMutableAttributedString, NSMutableDictionary,
    NSNumber, NSRange, NSString,
};
use objc2_ui_kit::UIView;
use objc2_ui_kit::{
    NSBaselineOffsetAttributeName, NSFontAttributeName, NSForegroundColorAttributeName,
    NSKernAttributeName, NSLineBreakMode, NSMutableParagraphStyle, NSParagraphStyleAttributeName,
    NSStrikethroughStyleAttributeName, NSTextAlignment, NSUnderlineStyleAttributeName, UILabel,
};

/// `NSUnderlineStyleSingle`, the value of the underline and strikethrough
/// attributes for one line.
const SINGLE_LINE: isize = 1;

/// Sets the string of a label and everything that decides its size: the
/// font, the spacing and the line limit.
///
/// Rendering and measuring both go through here, so that a frame is measured
/// for exactly what is drawn in it.
pub(crate) fn set_label_text(label: &UILabel, props: &TextProps) {
    let string = NSString::from_str(&props.content);
    let font = font_of(&props.font, props.size, props.weight, props.italic);

    if props.is_decorated() {
        let attributes = NSMutableDictionary::<NSAttributedStringKey, AnyObject>::new();
        let set = |key: &NSAttributedStringKey, value: Retained<AnyObject>| {
            // SAFETY: every value below has the type its key expects.
            unsafe {
                attributes.setObject_forKey(&value, objc2::runtime::ProtocolObject::from_ref(key))
            };
        };

        // SAFETY: the keys are constant strings exported by UIKit.
        unsafe {
            set(NSFontAttributeName, font.clone().into());
            set(NSForegroundColorAttributeName, color(props.color).into());
            if props.letter_spacing != 0.0 {
                let kern = NSNumber::new_f64(f64::from(props.letter_spacing));
                set(NSKernAttributeName, kern.into());
            }
            if props.underline {
                set(
                    NSUnderlineStyleAttributeName,
                    NSNumber::new_isize(SINGLE_LINE).into(),
                );
            }
            if props.strikethrough {
                set(
                    NSStrikethroughStyleAttributeName,
                    NSNumber::new_isize(SINGLE_LINE).into(),
                );
            }
            if let Some(line_height) = props.line_height {
                let line_height = f64::from(line_height);
                let paragraph = NSMutableParagraphStyle::new();
                paragraph.setMinimumLineHeight(line_height);
                paragraph.setMaximumLineHeight(line_height);
                set(NSParagraphStyleAttributeName, paragraph.into());
                // A fixed line height puts the text at the bottom of its
                // line; this moves it back to the middle.
                let offset = (line_height - font.lineHeight()) / 2.0;
                set(
                    NSBaselineOffsetAttributeName,
                    NSNumber::new_f64(offset).into(),
                );
            }
        }

        // SAFETY: the dictionary maps attribute names to values of the types
        // those attributes expect.
        let attributed = unsafe { NSAttributedString::new_with_attributes(&string, &attributes) };

        let runs = props.runs();
        if runs.is_empty() {
            label.setAttributedText(Some(&attributed));
        } else {
            // Each part of a rich text replaces the font, the colour and the
            // decorations of the whole in its own range.
            let rich = NSMutableAttributedString::from_attributed_nsstring(&attributed);
            let length = rich.length();
            for run in &runs {
                let start = (run.start as usize).min(length);
                let end = (run.end as usize).min(length);
                if start >= end {
                    continue;
                }
                let range = NSRange::new(start, end - start);
                let put = |key: &NSAttributedStringKey, value: Retained<AnyObject>| {
                    // SAFETY: the range lies inside the string, and every
                    // value below has the type its key expects.
                    unsafe { rich.addAttribute_value_range(key, &value, range) };
                };
                let line = |on: bool| NSNumber::new_isize(if on { SINGLE_LINE } else { 0 });

                // SAFETY: the keys are constant strings exported by UIKit.
                unsafe {
                    put(
                        NSFontAttributeName,
                        font_of(&run.font, run.size, run.weight, run.italic).into(),
                    );
                    put(NSForegroundColorAttributeName, color(run.color).into());
                    put(NSUnderlineStyleAttributeName, line(run.underline).into());
                    put(
                        NSStrikethroughStyleAttributeName,
                        line(run.strikethrough).into(),
                    );
                }
            }
            label.setAttributedText(Some(&rich));
        }
    } else {
        // A label keeps attributes of the text it had, so they are removed
        // before a plain string is set.
        label.setAttributedText(None);
        label.setText(Some(&string));
        // SAFETY: the font is retained by the label for as long as it is set.
        unsafe { label.setFont(Some(&font)) };
    }

    // These apply to the whole text, plain or attributed, and are set last.
    label.setNumberOfLines(props.max_lines.map_or(0, |lines| lines.max(1) as isize));
    label.setLineBreakMode(if props.ellipsis {
        NSLineBreakMode::ByTruncatingTail
    } else {
        NSLineBreakMode::ByWordWrapping
    });
}

pub(crate) fn update_label(label: &UILabel, props: &TextProps) {
    set_label_text(label, props);
    if !props.is_decorated() {
        let text_color = color(props.color);
        // SAFETY: the color is retained by the label for as long as it is set.
        unsafe { label.setTextColor(Some(&text_color)) };
    }
    label.setTextAlignment(match props.align {
        TextAlign::Start => NSTextAlignment(0),
        TextAlign::Center => NSTextAlignment(1),
        TextAlign::End => NSTextAlignment(2),
    });
}

/// Creates the native view of a text.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let wrapper = UIView::new(site.mtm);
    let label = UILabel::new(site.mtm);
    wrapper.addSubview(&label);
    if let crate::tree::NodeKind::Widget(crate::widgets::Widget::Text(props)) = &node.kind
        && props.selectable
    {
        copy::attach(&wrapper, &label, site.mtm);
    }
    ViewHandle::new(wrapper, Content::Label(label), node)
}

/// Copying a text: a long press on it offers "Copy".
///
/// A `UILabel` cannot be selected, and a `UITextView` does not measure as a
/// label does. So a selectable text stays a label, which is measured and
/// drawn as every other text, and the whole of it can be copied.
mod copy {
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
    use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_core_foundation::CGPoint;
    use objc2_foundation::{NSArray, NSString};
    use objc2_ui_kit::{
        UIAction, UIContextMenuConfiguration, UIContextMenuInteraction,
        UIContextMenuInteractionDelegate, UIImage, UILabel, UIMenu, UIMenuElement, UIPasteboard,
        UIView,
    };
    use std::ptr::NonNull;

    pub(super) struct Ivars {
        label: Retained<UILabel>,
    }

    objc2::define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "KrabikCopyMenu"]
        #[ivars = Ivars]
        pub(super) struct CopyMenu;

        unsafe impl NSObjectProtocol for CopyMenu {}

        unsafe impl UIContextMenuInteractionDelegate for CopyMenu {
            #[unsafe(method_id(contextMenuInteraction:configurationForMenuAtLocation:))]
            fn configuration(
                &self,
                _interaction: &UIContextMenuInteraction,
                _location: CGPoint,
            ) -> Option<Retained<UIContextMenuConfiguration>> {
                Some(configuration(self.ivars().label.clone(), self.mtm()))
            }
        }
    );

    fn configuration(
        label: Retained<UILabel>,
        mtm: MainThreadMarker,
    ) -> Retained<UIContextMenuConfiguration> {
        let provider = RcBlock::new(move |_suggested: NonNull<NSArray<UIMenuElement>>| {
            let label = label.clone();
            let copy: RcBlock<dyn Fn(NonNull<UIAction>)> = RcBlock::new(move |_| {
                let text = label.text().unwrap_or_default();
                // SAFETY: the string is copied by the pasteboard.
                unsafe { UIPasteboard::generalPasteboard().setString(Some(&text)) };
            });
            let image = UIImage::systemImageNamed(&NSString::from_str("doc.on.doc"));
            // SAFETY: the block is a valid `UIActionHandler` and is retained
            // by the action.
            let action = unsafe {
                UIAction::actionWithTitle_image_identifier_handler(
                    &NSString::from_str("Copy"),
                    image.as_deref(),
                    None,
                    RcBlock::as_ptr(&copy),
                    mtm,
                )
            };
            let lines = [Retained::into_super(action)];
            Retained::autorelease_ptr(UIMenu::menuWithChildren(
                &NSArray::from_retained_slice(&lines),
                mtm,
            ))
        });
        // SAFETY: the provider is a valid block that returns a menu the
        // caller takes over; the configuration copies the block.
        unsafe {
            UIContextMenuConfiguration::configurationWithIdentifier_previewProvider_actionProvider(
                None,
                std::ptr::null_mut(),
                RcBlock::as_ptr(&provider),
                mtm,
            )
        }
    }

    /// The key the view keeps its delegate under.
    static KEY: u8 = 0;

    /// Lets a long press on `view` copy the text of `label`.
    pub(super) fn attach(view: &UIView, label: &Retained<UILabel>, mtm: MainThreadMarker) {
        let delegate = CopyMenu::alloc(mtm).set_ivars(Ivars {
            label: label.clone(),
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance
        // of this class with the ivars already set.
        let delegate: Retained<CopyMenu> = unsafe { objc2::msg_send![super(delegate), init] };
        let interaction = UIContextMenuInteraction::initWithDelegate(
            UIContextMenuInteraction::alloc(mtm),
            ProtocolObject::from_ref(&*delegate),
        );
        view.addInteraction(ProtocolObject::from_ref(&*interaction));
        // The interaction does not retain its delegate; the view does, for
        // as long as it lives. The delegate holds the label, not the view,
        // so nothing holds itself.
        // SAFETY: both objects are live, and the key is a unique address.
        unsafe {
            objc2::ffi::objc_setAssociatedObject(
                view as *const UIView as *mut AnyObject,
                std::ptr::addr_of!(KEY).cast(),
                Retained::as_ptr(&delegate) as *mut AnyObject,
                objc2::ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
    }
}
