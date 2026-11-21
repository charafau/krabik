//! Fonts on iOS: the system font in a weight, or a font file from the bundle.

use crate::ios::images::asset_path;
use crate::ui::{Font, FontWeight};
use objc2::rc::Retained;
use objc2_foundation::{NSArray, NSURL};
use objc2_ui_kit::{UIFont, UIFontDescriptor, UIFontDescriptorSymbolicTraits};
use std::cell::RefCell;
use std::collections::HashMap;

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    /// `CFArrayRef CTFontManagerCreateFontDescriptorsFromURL(CFURLRef)`: the
    /// faces in a font file, or null when it cannot be read.
    ///
    /// `CFURL`, `CFArray` and `CTFontDescriptor` are toll-free bridged with
    /// `NSURL`, `NSArray` and `UIFontDescriptor`, which is how they are
    /// declared here. The array follows the create rule: the caller owns it.
    fn CTFontManagerCreateFontDescriptorsFromURL(
        file_url: *const NSURL,
    ) -> *mut NSArray<UIFontDescriptor>;
}

thread_local! {
    /// The font files that were opened, by asset path; `None` for a file
    /// that could not be loaded, so that it is tried and reported once.
    static FONT_FILES: RefCell<HashMap<String, Option<Retained<UIFontDescriptor>>>> =
        RefCell::new(HashMap::new());
}

/// The font of a text: the file it names, as it is, or the system font in
/// the given weight and slant.
pub(crate) fn font_of(
    font: &Font,
    size: f32,
    weight: FontWeight,
    italic: bool,
) -> Retained<UIFont> {
    font.asset_path()
        .and_then(|path| file_font(path, size))
        .unwrap_or_else(|| self::font(size, weight, italic))
}

/// The system font in the given weight, upright.
pub(crate) fn system_font(size: f32, weight: FontWeight) -> Retained<UIFont> {
    font(size, weight, false)
}

/// The system font in the given weight, upright or italic.
pub(crate) fn font(size: f32, weight: FontWeight, italic: bool) -> Retained<UIFont> {
    let size = f64::from(size);
    let upright = UIFont::systemFontOfSize_weight(size, ui_font_weight(weight));
    if !italic {
        return upright;
    }

    // SAFETY: fonts are immutable, reading the descriptor of one that was
    // just created on this thread is safe.
    let descriptor = unsafe { upright.fontDescriptor() };
    // Asking for the italic trait alone gives the regular italic, whatever
    // the weight was. The system font has an italic in regular and in bold,
    // so a weight from semi-bold on gets the bold one.
    let mut traits = UIFontDescriptorSymbolicTraits::TraitItalic;
    if weight.is_bold() {
        traits |= UIFontDescriptorSymbolicTraits::TraitBold;
    }
    match descriptor.fontDescriptorWithSymbolicTraits(traits) {
        // a size of zero keeps the size of the descriptor
        Some(italic) => UIFont::fontWithDescriptor_size(&italic, 0.0),
        None => upright,
    }
}

/// The values of UIKit's `UIFontWeight` constants, `UIFontWeightUltraLight`
/// to `UIFontWeightBlack`.
fn ui_font_weight(weight: FontWeight) -> f64 {
    match weight {
        FontWeight::Thin => -0.8,
        FontWeight::ExtraLight => -0.6,
        FontWeight::Light => -0.4,
        FontWeight::Normal => 0.0,
        FontWeight::Medium => 0.23,
        FontWeight::SemiBold => 0.3,
        FontWeight::Bold => 0.4,
        FontWeight::ExtraBold => 0.56,
        FontWeight::Black => 0.62,
    }
}

/// The first face of a font file in the bundle, at the given size.
fn file_font(path: &str, size: f32) -> Option<Retained<UIFont>> {
    FONT_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let descriptor = files.entry(path.to_owned()).or_insert_with(|| {
            let descriptor = load_descriptor(path);
            if descriptor.is_none() {
                crate::log::error(&format!("font {path} could not be loaded"));
            }
            descriptor
        });
        descriptor
            .as_ref()
            .map(|descriptor| UIFont::fontWithDescriptor_size(descriptor, f64::from(size)))
    })
}

fn load_descriptor(path: &str) -> Option<Retained<UIFontDescriptor>> {
    let file = asset_path(path)?;
    let url = NSURL::fileURLWithPath(&file);
    // SAFETY: the URL is valid for the call, and the returned array, if any,
    // is owned by the caller; see the declaration.
    let faces = unsafe {
        Retained::from_raw(CTFontManagerCreateFontDescriptorsFromURL(Retained::as_ptr(
            &url,
        )))
    }?;
    faces.firstObject()
}
