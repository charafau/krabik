//! The `container` widget on iOS.

use crate::ios::ViewHandle;
use crate::ios::events::dispatch_click;
use crate::ios::schedule;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::tree::NodeId;
use crate::ui::ContainerProps;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_ui_kit::{NSObjectUIAccessibility, UITapGestureRecognizer, UIView};

/// The node a tap target reports for.
pub(crate) struct TapTargetIvars {
    handle: u64,
    host: u64,
    id: NodeId,
}

objc2::define_class!(
    // A gesture recognizer calls a selector on a target object; a plain
    // `UIView` has no block-based action as a `UIControl` does.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikTapTarget"]
    #[ivars = TapTargetIvars]
    pub(crate) struct TapTarget;

    unsafe impl NSObjectProtocol for TapTarget {}

    impl TapTarget {
        #[unsafe(method(tapped:))]
        fn tapped(&self, _recognizer: &UITapGestureRecognizer) {
            let ivars = self.ivars();
            let (handle, host, id) = (ivars.handle, ivars.host, ivars.id);
            schedule(move || dispatch_click(handle, host, id));
        }
    }
);

impl TapTarget {
    fn new(site: &Site) -> Retained<Self> {
        let this = Self::alloc(site.mtm).set_ivars(TapTargetIvars {
            handle: site.handle,
            host: site.host,
            id: site.node,
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }
}

/// Creates the native view of a container: a view its children are added
/// to, with a tap recognizer that [`update_container`] turns on and off.
///
/// UIKit does not let a tap recognizer on a superview begin on a control
/// such as a button or a switch, so a control inside keeps its own taps.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let view = UIView::new(site.mtm);
    let host = UIView::new(site.mtm);
    view.addSubview(&host);

    let target = TapTarget::new(site);
    // SAFETY: `tapped:` is a method of the target, which takes the
    // recognizer as its only argument. The recognizer does not retain its
    // target; the handle keeps it for as long as the view exists.
    let tap: Retained<UITapGestureRecognizer> = unsafe {
        objc2::msg_send![
            UITapGestureRecognizer::alloc(site.mtm),
            initWithTarget: &*target,
            action: objc2::sel!(tapped:),
        ]
    };
    view.addGestureRecognizer(&tap);
    ViewHandle::new(
        view,
        Content::Container {
            host,
            tap,
            _target: target,
        },
        node,
    )
}

/// Lets the container react to taps while it has a handler.
pub(crate) fn update_container(tap: &UITapGestureRecognizer, props: &ContainerProps) {
    tap.setEnabled(props.on_tap.is_set());
}

/// Announces a tappable container as a button to assistive technologies.
pub(crate) fn apply_traits(view: &UIView, props: &ContainerProps, mtm: MainThreadMarker) {
    // SAFETY: both are constant bit sets exported by UIKit.
    let traits = unsafe {
        if props.on_tap.is_set() {
            objc2_ui_kit::UIAccessibilityTraitButton
        } else {
            objc2_ui_kit::UIAccessibilityTraitNone
        }
    };
    view.setAccessibilityTraits(traits, mtm);
}
