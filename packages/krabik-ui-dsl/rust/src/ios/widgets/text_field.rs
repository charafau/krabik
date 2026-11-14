//! The `text_field` widget on iOS.

use crate::ios::ViewHandle;
use crate::ios::events::{dispatch_focus, dispatch_submitted, dispatch_text};
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{add_action, color, schedule, system_font};
use crate::tree::NodeId;
use crate::ui::{ImeAction, KeyboardType, TextFieldProps};
use objc2::MainThreadMarker;
use objc2::runtime::{AnyObject, Sel};
use objc2_foundation::NSString;
use objc2_ui_kit::{UIControlEvents, UIKeyboardType, UIReturnKeyType, UITextField};
use objc2_ui_kit::{UITextBorderStyle, UITextInputTraits, UIView};

pub(crate) fn update_field(field: &UITextField, props: &TextFieldProps) {
    let current = field
        .text()
        .map(|text| text.to_string())
        .unwrap_or_default();
    if current != props.value {
        field.setText(Some(&NSString::from_str(&props.value)));
    }
    // The remaining setters reconfigure the text input system, which can
    // disturb active typing; skip them when the values already match so a
    // re-render between keystrokes stays a no-op.
    let placeholder = field.placeholder().map(|text| text.to_string());
    if placeholder.as_deref() != Some(props.placeholder.as_str()) {
        field.setPlaceholder(Some(&NSString::from_str(&props.placeholder)));
    }
    field.setFont(Some(&system_font(props.size, props.weight)));
    field.setTextColor(Some(&color(props.color)));
    let keyboard_type = match props.keyboard {
        KeyboardType::Text => UIKeyboardType::Default,
        KeyboardType::Email => UIKeyboardType::EmailAddress,
        KeyboardType::Number => UIKeyboardType::NumberPad,
        KeyboardType::Phone => UIKeyboardType::PhonePad,
    };
    let return_key_type = match props.ime_action {
        ImeAction::Default => UIReturnKeyType::Default,
        ImeAction::Done => UIReturnKeyType::Done,
        ImeAction::Next => UIReturnKeyType::Next,
        ImeAction::Search => UIReturnKeyType::Search,
        ImeAction::Send => UIReturnKeyType::Send,
    };
    let (current_keyboard, current_return_key, current_secure) = text_input_traits(field);
    if current_keyboard != keyboard_type.0 || current_return_key != return_key_type.0 {
        set_text_input_traits(field, keyboard_type, return_key_type);
    }
    if current_secure != props.secure {
        field.setSecureTextEntry(props.secure);
    }
}

/// Reads the keyboard, return-key, and secure-entry traits through the raw
/// runtime.
///
/// Mirrors [`set_text_input_traits`]: `UITextField` does not declare these
/// `UITextInputTraits` accessors itself, so objc2's debug-time method
/// verification rejects the generated getters even though messaging works.
pub(crate) fn text_input_traits(field: &UITextField) -> (isize, isize, bool) {
    let receiver = field as *const UITextField as *mut AnyObject;
    // SAFETY: all three getters take no arguments and return an integer
    // (`NSInteger` / `BOOL`), which arrives in `x0` on arm64.
    unsafe {
        let send_integer: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) -> isize =
            std::mem::transmute(objc2::ffi::objc_msgSend as *const ());
        let send_bool: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) -> i8 =
            std::mem::transmute(objc2::ffi::objc_msgSend as *const ());
        (
            send_integer(receiver, objc2::sel!(keyboardType)),
            send_integer(receiver, objc2::sel!(returnKeyType)),
            send_bool(receiver, objc2::sel!(isSecureTextEntry)) != 0,
        )
    }
}

/// Sets the keyboard and return-key traits through the raw runtime.
///
/// UIKit implements these `UITextInputTraits` setters through message
/// forwarding, so objc2's debug-time method lookup does not find them even
/// though messaging works.
pub(crate) fn set_text_input_traits(
    field: &UITextField,
    keyboard_type: UIKeyboardType,
    return_key_type: UIReturnKeyType,
) {
    let receiver = field as *const UITextField as *mut AnyObject;
    // SAFETY: both setters take one `NSInteger` argument and return `void`.
    unsafe {
        let send: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, isize) =
            std::mem::transmute(objc2::ffi::objc_msgSend as *const ());
        send(receiver, objc2::sel!(setKeyboardType:), keyboard_type.0);
        send(receiver, objc2::sel!(setReturnKeyType:), return_key_type.0);
    }
}

pub(crate) fn wire_field(
    field: &UITextField,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        field,
        UIControlEvents::EditingChanged,
        "krabik.text",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UITextField>().ok())
                .and_then(|field| field.text())
                .map(|text| text.to_string())
                .unwrap_or_default();
            schedule(move || dispatch_text(handle, host, id, &value));
        },
    );
    add_action(
        field,
        UIControlEvents::EditingDidEndOnExit,
        "krabik.submit",
        mtm,
        move |_| schedule(move || dispatch_submitted(handle, host, id)),
    );
    add_action(
        field,
        UIControlEvents::EditingDidBegin,
        "krabik.focus-begin",
        mtm,
        move |_| schedule(move || dispatch_focus(handle, host, id, true)),
    );
    add_action(
        field,
        UIControlEvents::EditingDidEnd,
        "krabik.focus-end",
        mtm,
        move |_| schedule(move || dispatch_focus(handle, host, id, false)),
    );
}

/// Creates the native view of a text field.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let wrapper = UIView::new(site.mtm);
    let field = UITextField::new(site.mtm);
    field.setBorderStyle(UITextBorderStyle::None);
    wire_field(&field, site.handle, site.host, site.node, site.mtm);
    wrapper.addSubview(&field);
    ViewHandle::new(wrapper, Content::Field(field), node)
}
