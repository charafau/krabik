//! The `native_view` widget on iOS.
//!
//! The views come from `KrabikNativeViews`, a Swift class of the shared host
//! sources that holds the application's factories. It is found by name and
//! called through its Objective-C methods.

use crate::host::NodeEvent;
use crate::ios::controller::string_array;
use crate::ios::events::dispatch;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, schedule};
use crate::ui::NativeViewProps;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2_foundation::NSString;
use objc2_ui_kit::UIView;
use std::ptr::NonNull;

fn registry() -> Option<&'static AnyClass> {
    let class = AnyClass::get(c"KrabikNativeViews");
    if class.is_none() {
        crate::log::error(
            "KrabikNativeViews is not in the app; add shared/ios/KrabikNativeViews.swift",
        );
    }
    class
}

/// Tells the host its name and the properties of the description; the host
/// asks the application's factory for a view when the name is new.
pub(crate) fn update_native_view(host: &UIView, props: &NativeViewProps) {
    let Some(registry) = registry() else {
        return;
    };
    let name = NSString::from_str(&props.name);
    let keys = string_array(props.props.iter().map(|(key, _)| key.as_str()));
    let values = string_array(props.props.iter().map(|(_, value)| value.as_str()));
    // SAFETY: the class has this method with these argument types.
    let _: () = unsafe {
        objc2::msg_send![registry, update: host, name: &*name, keys: &*keys, values: &*values]
    };
}

/// Creates the host of a native view: an empty view until the first update.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let (handle, host, id) = (site.handle, site.host, site.node);
    let emit = RcBlock::new(move |value: NonNull<NSString>| {
        // SAFETY: the block is called with a live string.
        let value = unsafe { value.as_ref() }.to_string();
        schedule(move || dispatch(handle, host, id, NodeEvent::Native(value)));
    });
    let view: Retained<UIView> = match registry() {
        // SAFETY: the class has this method; it copies the block and
        // returns a view.
        Some(registry) => unsafe { objc2::msg_send![registry, makeHost: &*emit] },
        None => UIView::new(site.mtm),
    };
    ViewHandle::new(view.clone(), Content::NativeView(view), node)
}
