//! Tooltips on iOS.
//!
//! UIKit shows a tooltip only for a pointer. On a touch screen a long press
//! shows a small label above the view for a moment, which is what Android's
//! own tooltip does.

use crate::ios::color;
use crate::ui::Color;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSString;
use objc2_ui_kit::{
    NSTextAlignment, UIFont, UIGestureRecognizerState, UILabel, UILongPressGestureRecognizer,
    UIView, UIViewAnimationOptions,
};
use std::cell::RefCell;

/// How long the label stays before it fades, in seconds.
const SHOWN_FOR: f64 = 1.8;
/// The space between the label and the view it explains, and around its
/// text, in points.
const GAP: f64 = 6.0;
const PADDING_X: f64 = 10.0;
const PADDING_Y: f64 = 6.0;

pub(crate) struct TargetIvars {
    text: RefCell<String>,
}

objc2::define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikTooltipTarget"]
    #[ivars = TargetIvars]
    pub(crate) struct Target;

    unsafe impl NSObjectProtocol for Target {}

    impl Target {
        #[unsafe(method(held:))]
        fn held(&self, recognizer: &UILongPressGestureRecognizer) {
            if recognizer.state() != UIGestureRecognizerState::Began {
                return;
            }
            if let Some(view) = recognizer.view() {
                show(&view, &self.ivars().text.borrow(), self.mtm());
            }
        }
    }
);

/// What a view with a tooltip keeps: the recognizer does not retain its
/// target.
pub(crate) struct Tooltip {
    target: Retained<Target>,
    recognizer: Retained<UILongPressGestureRecognizer>,
}

/// Gives `view` the tooltip `text`, changes it, or takes it away.
pub(crate) fn apply(
    view: &UIView,
    slot: &mut Option<Tooltip>,
    text: Option<&str>,
    mtm: MainThreadMarker,
) {
    match (text, slot.as_ref()) {
        (Some(text), Some(tooltip)) => {
            let mut current = tooltip.target.ivars().text.borrow_mut();
            if *current != text {
                *current = text.to_owned();
            }
        }
        (Some(text), None) => {
            let target = Target::alloc(mtm).set_ivars(TargetIvars {
                text: RefCell::new(text.to_owned()),
            });
            // SAFETY: `NSObject`'s initializer returns an initialized
            // instance of this class with the ivars already set.
            let target: Retained<Target> = unsafe { objc2::msg_send![super(target), init] };
            // SAFETY: `held:` is a method of the target that takes the
            // recognizer; the slot keeps the target alive with the view.
            let recognizer: Retained<UILongPressGestureRecognizer> = unsafe {
                objc2::msg_send![
                    UILongPressGestureRecognizer::alloc(mtm),
                    initWithTarget: &*target,
                    action: objc2::sel!(held:),
                ]
            };
            view.addGestureRecognizer(&recognizer);
            *slot = Some(Tooltip { target, recognizer });
        }
        (None, Some(_)) => {
            if let Some(tooltip) = slot.take() {
                view.removeGestureRecognizer(&tooltip.recognizer);
            }
        }
        (None, None) => {}
    }
}

/// Shows `text` above `view` (below it when there is no room), fades it in,
/// and fades it out again after a moment.
fn show(view: &UIView, text: &str, mtm: MainThreadMarker) {
    let Some(window) = view.window() else {
        return;
    };
    let anchor = view.convertRect_toView(view.bounds(), None);
    let bounds = window.bounds();

    let label = UILabel::new(mtm);
    label.setText(Some(&NSString::from_str(text)));
    label.setNumberOfLines(0);
    label.setTextAlignment(NSTextAlignment::Center);
    // SAFETY: the font and the colors are retained by the label.
    unsafe {
        label.setFont(Some(&UIFont::systemFontOfSize(13.0)));
        label.setTextColor(Some(&color(Color::fixed(0xFFFF_FFFF))));
    }
    label.setBackgroundColor(Some(&color(Color::fixed(0xE632_3232))));
    label.layer().setCornerRadius(6.0);
    label.setClipsToBounds(true);
    label.setUserInteractionEnabled(false);

    let widest = (bounds.size.width - 16.0 - PADDING_X * 2.0).clamp(40.0, 260.0);
    let fitted = label.sizeThatFits(CGSize::new(widest, f64::MAX));
    let width = fitted.width.min(widest).ceil() + PADDING_X * 2.0;
    let height = fitted.height.ceil() + PADDING_Y * 2.0;
    let middle = anchor.origin.x + anchor.size.width / 2.0;
    let x = (middle - width / 2.0).clamp(8.0, (bounds.size.width - width - 8.0).max(8.0));
    let above = anchor.origin.y - height - GAP;
    let y = if above >= window.safeAreaInsets().top {
        above
    } else {
        anchor.origin.y + anchor.size.height + GAP
    };
    label.setFrame(CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)));
    label.setAlpha(0.0);
    window.addSubview(&label);

    // The label fades in, and only then is told to fade out: two changes of
    // one property made in the same turn would leave it at the last value.
    let appearing = label.clone();
    let appear = RcBlock::new(move || appearing.setAlpha(1.0));
    let stay = RcBlock::new(move |_: Bool| {
        let leaving = label.clone();
        let leave = RcBlock::new(move || leaving.setAlpha(0.0));
        let removed = label.clone();
        let remove = RcBlock::new(move |_: Bool| removed.removeFromSuperview());
        UIView::animateWithDuration_delay_options_animations_completion(
            0.25,
            SHOWN_FOR,
            UIViewAnimationOptions::empty(),
            &leave,
            Some(&remove),
            mtm,
        );
    });
    UIView::animateWithDuration_delay_options_animations_completion(
        0.15,
        0.0,
        UIViewAnimationOptions::empty(),
        &appear,
        Some(&stay),
        mtm,
    );
}
