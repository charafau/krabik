//! The `container` widget on iOS.

use crate::host::NodeEvent;
use crate::ios::ViewHandle;
use crate::ios::events::dispatch_selected;
use crate::ios::events::{dispatch, dispatch_click};
use crate::ios::schedule;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::tree::NodeId;
use crate::ui::ContainerProps;
use crate::ui::MenuProps;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::CGPoint;
use objc2_foundation::{NSArray, NSString};
use objc2_ui_kit::{
    NSObjectUIAccessibility, UIAction, UIButton, UIButtonType, UIContextMenuConfiguration,
    UIContextMenuInteraction, UIContextMenuInteractionDelegate, UIGestureRecognizerState,
    UILongPressGestureRecognizer, UIMenu, UIMenuElement, UIMenuElementAttributes,
    UIMenuElementState, UITapGestureRecognizer, UIView, UIViewAutoresizing,
};
use std::cell::RefCell;
use std::ptr::NonNull;

/// The node a tap target reports for.
pub(crate) struct TapTargetIvars {
    handle: u64,
    host: u64,
    id: NodeId,
    /// The menu a long press opens, while the container has one.
    context_menu: RefCell<Option<Retained<UIMenu>>>,
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

    unsafe impl UIContextMenuInteractionDelegate for TapTarget {
        #[unsafe(method_id(contextMenuInteraction:configurationForMenuAtLocation:))]
        fn menu_configuration(
            &self,
            _interaction: &UIContextMenuInteraction,
            _location: CGPoint,
        ) -> Option<Retained<UIContextMenuConfiguration>> {
            let menu = self.ivars().context_menu.borrow().clone();
            menu.map(|menu| menu_configuration(menu, self.mtm()))
        }
    }

    impl TapTarget {
        #[unsafe(method(tapped:))]
        fn tapped(&self, _recognizer: &UITapGestureRecognizer) {
            let ivars = self.ivars();
            let (handle, host, id) = (ivars.handle, ivars.host, ivars.id);
            schedule(move || dispatch_click(handle, host, id));
        }

        #[unsafe(method(held:))]
        fn held(&self, recognizer: &UILongPressGestureRecognizer) {
            // The recognizer keeps reporting while the finger moves and when
            // it lifts; the press counts once, when it is recognized.
            if recognizer.state() != UIGestureRecognizerState::Began {
                return;
            }
            let ivars = self.ivars();
            let (handle, host, id) = (ivars.handle, ivars.host, ivars.id);
            schedule(move || dispatch(handle, host, id, NodeEvent::LongPress));
        }
    }
);

/// A context menu configuration that shows `menu`.
fn menu_configuration(
    menu: Retained<UIMenu>,
    mtm: MainThreadMarker,
) -> Retained<UIContextMenuConfiguration> {
    let provider = RcBlock::new(move |_suggested: NonNull<NSArray<UIMenuElement>>| {
        Retained::autorelease_ptr(menu.clone())
    });
    // SAFETY: the provider is a valid block that returns a menu the caller
    // takes over; the configuration copies the block.
    unsafe {
        UIContextMenuConfiguration::configurationWithIdentifier_previewProvider_actionProvider(
            None,
            std::ptr::null_mut(),
            RcBlock::as_ptr(&provider),
            mtm,
        )
    }
}

impl TapTarget {
    fn new(site: &Site) -> Retained<Self> {
        let this = Self::alloc(site.mtm).set_ivars(TapTargetIvars {
            handle: site.handle,
            host: site.host,
            id: site.node,
            context_menu: RefCell::new(None),
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
    // SAFETY: as above, for `held:`.
    let long_press: Retained<UILongPressGestureRecognizer> = unsafe {
        objc2::msg_send![
            UILongPressGestureRecognizer::alloc(site.mtm),
            initWithTarget: &*target,
            action: objc2::sel!(held:),
        ]
    };
    view.addGestureRecognizer(&long_press);
    ViewHandle::new(
        view,
        Content::Container {
            host,
            tap,
            long_press,
            menu_button: None,
            context_menu: None,
            target,
        },
        node,
    )
}

/// The parts of a container's handle that its properties change.
pub(crate) struct Parts<'a> {
    pub(crate) host: &'a UIView,
    pub(crate) tap: &'a UITapGestureRecognizer,
    pub(crate) long_press: &'a UILongPressGestureRecognizer,
    pub(crate) menu_button: &'a mut Option<Retained<UIButton>>,
    pub(crate) context_menu: &'a mut Option<Retained<UIContextMenuInteraction>>,
    pub(crate) target: &'a TapTarget,
}

/// Lets the container react to taps and long presses while it has a handler
/// for them, and gives it the menu it opens.
pub(crate) fn update_container(parts: Parts<'_>, props: &ContainerProps, mtm: MainThreadMarker) {
    let menu_on = |long_press: bool| {
        props
            .menu
            .as_ref()
            .filter(|menu| menu.on_long_press == long_press)
    };
    parts.tap.setEnabled(props.on_tap.is_set());
    // The system's own recognizer opens a context menu.
    parts
        .long_press
        .setEnabled(props.on_long_press.is_set() && menu_on(true).is_none());
    let Some(view) = parts.host.superview() else {
        return;
    };
    let ivars = parts.target.ivars();
    let build = |menu: &MenuProps| build_menu(menu, ivars.handle, ivars.host, ivars.id, mtm);

    // A menu that opens on a tap belongs to a button, the one kind of view
    // UIKit opens a menu from without a long press. A clear button over the
    // container is that button.
    match menu_on(false) {
        Some(menu) => {
            let button = parts.menu_button.get_or_insert_with(|| {
                let button = UIButton::buttonWithType(UIButtonType::Custom, mtm);
                button.setFrame(view.bounds());
                button.setAutoresizingMask(
                    UIViewAutoresizing::FlexibleWidth | UIViewAutoresizing::FlexibleHeight,
                );
                button.setShowsMenuAsPrimaryAction(true);
                view.addSubview(&button);
                button
            });
            button.setMenu(Some(&build(menu)));
        }
        None => {
            if let Some(button) = parts.menu_button.take() {
                button.removeFromSuperview();
            }
        }
    }

    match menu_on(true) {
        Some(menu) => {
            *ivars.context_menu.borrow_mut() = Some(build(menu));
            if parts.context_menu.is_none() {
                let interaction = UIContextMenuInteraction::initWithDelegate(
                    UIContextMenuInteraction::alloc(mtm),
                    ProtocolObject::from_ref(parts.target),
                );
                view.addInteraction(ProtocolObject::from_ref(&*interaction));
                *parts.context_menu = Some(interaction);
            }
        }
        None => {
            ivars.context_menu.borrow_mut().take();
            if let Some(interaction) = parts.context_menu.take() {
                view.removeInteraction(ProtocolObject::from_ref(&*interaction));
            }
        }
    }
}

/// The platform's menu for `menu`; each line reports its index.
fn build_menu(
    menu: &MenuProps,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) -> Retained<UIMenu> {
    let lines: Vec<Retained<UIMenuElement>> = menu
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let block: RcBlock<dyn Fn(NonNull<UIAction>)> = RcBlock::new(move |_| {
                schedule(move || dispatch_selected(handle, host, id, index));
            });
            let image = item
                .icon
                .as_ref()
                .and_then(|icon| icon.ios_name())
                .and_then(crate::ios::images::load_icon);
            // SAFETY: the block is a valid `UIActionHandler` and is retained
            // by the action.
            let action = unsafe {
                UIAction::actionWithTitle_image_identifier_handler(
                    &NSString::from_str(&item.label),
                    image.as_deref(),
                    None,
                    RcBlock::as_ptr(&block),
                    mtm,
                )
            };
            if item.destructive {
                action.setAttributes(UIMenuElementAttributes::Destructive);
            }
            if menu.selected == Some(index) {
                action.setState(UIMenuElementState::On);
            }
            Retained::into_super(action)
        })
        .collect();
    UIMenu::menuWithChildren(&NSArray::from_retained_slice(&lines), mtm)
}

/// Announces a tappable container as a button to assistive technologies.
pub(crate) fn apply_traits(view: &UIView, props: &ContainerProps, mtm: MainThreadMarker) {
    // SAFETY: both are constant bit sets exported by UIKit.
    let traits = unsafe {
        if props.on_tap.is_set() || props.on_long_press.is_set() || props.menu.is_some() {
            objc2_ui_kit::UIAccessibilityTraitButton
        } else {
            objc2_ui_kit::UIAccessibilityTraitNone
        }
    };
    view.setAccessibilityTraits(traits, mtm);
}
