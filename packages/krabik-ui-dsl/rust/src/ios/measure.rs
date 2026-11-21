//! Text and control measurement through off-screen UIKit views.

use crate::geometry::{Constraints, Size};
use crate::ios::images::load_asset;
use crate::ios::widgets::text::set_label_text;
use crate::measure::{DEFAULT_SWITCH_HEIGHT, DEFAULT_SWITCH_WIDTH, Measure};
use crate::ui::{ImageProps, ImageSource, TextProps};
use crate::widgets::TextMeasureKey;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_core_foundation::CGSize;
use objc2_ui_kit::{UILabel, UISwitch};
use std::collections::HashMap;
use std::sync::Mutex;

/// Measures text with an off-screen `UILabel`.
///
/// Results are cached because Taffy asks for the same intrinsic sizes several
/// times per layout pass. UIKit objects are only touched from the main thread,
/// which is also where the runtime renders.
pub(crate) struct IosMeasure {
    pub(crate) label: Retained<UILabel>,
    pub(crate) cache: Mutex<HashMap<MeasureKey, Size>>,
    pub(crate) switch_size: Mutex<Option<Size>>,
    pub(crate) image_sizes: Mutex<HashMap<String, Option<Size>>>,
}

// SAFETY: every method is called from the main thread by `IosState::render`.
unsafe impl Send for IosMeasure {}
unsafe impl Sync for IosMeasure {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MeasureKey {
    /// Everything about the text that decides its size.
    pub(crate) text: TextMeasureKey,
    pub(crate) max_width: Option<u32>,
}

impl IosMeasure {
    pub(crate) fn new(mtm: MainThreadMarker) -> Self {
        let label = UILabel::new(mtm);
        label.setAdjustsFontSizeToFitWidth(false);
        Self {
            label,
            cache: Mutex::new(HashMap::new()),
            switch_size: Mutex::new(None),
            image_sizes: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn measure_with(&self, text: &TextProps, constraints: Constraints) -> Size {
        if text.content.is_empty() {
            return Size::ZERO;
        }

        let key = MeasureKey {
            text: text.measure_key(),
            max_width: constraints.max_width.map(f32::to_bits),
        };

        if let Some(cached) = self
            .cache
            .lock()
            .expect("measurement cache")
            .get(&key)
            .copied()
        {
            return cached;
        }

        let label = &self.label;
        set_label_text(label, text);

        let max_width = constraints.max_width.map_or(f64::MAX, f64::from);
        let measured = label.sizeThatFits(CGSize::new(max_width, f64::MAX));
        let size = Size::new(measured.width.ceil() as f32, measured.height.ceil() as f32);

        self.cache
            .lock()
            .expect("measurement cache")
            .insert(key, size);
        size
    }
}

impl Measure for IosMeasure {
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
        self.measure_with(text, constraints)
    }

    fn switch_size(&self) -> Size {
        if let Some(size) = *self.switch_size.lock().expect("switch size cache") {
            return size;
        }
        // The switch must be created on the main thread; layout already runs
        // there. A host build falls back to the Material default.
        let size = MainThreadMarker::new()
            .map(UISwitch::new)
            .map(|switch| switch.intrinsicContentSize())
            .filter(|size| size.width > 0.0 && size.height > 0.0)
            .map_or(
                Size::new(DEFAULT_SWITCH_WIDTH, DEFAULT_SWITCH_HEIGHT),
                |size| Size::new(size.width as f32, size.height as f32),
            );
        *self.switch_size.lock().expect("switch size cache") = Some(size);
        size
    }

    fn image_size(&self, props: &ImageProps) -> Option<Size> {
        let ImageSource::Asset(path) = &props.source else {
            return None;
        };
        if let Some(cached) = self
            .image_sizes
            .lock()
            .expect("image size cache")
            .get(path)
            .copied()
        {
            return cached;
        }
        #[cfg(feature = "svg")]
        if crate::svg::is_svg(path) {
            // An SVG says its size in points; no image has to be decoded.
            let size = crate::ios::images::asset_bytes(path)
                .and_then(|data| crate::svg::natural_size(&data));
            self.image_sizes
                .lock()
                .expect("image size cache")
                .insert(path.clone(), size);
            return size;
        }
        let size = MainThreadMarker::new().and_then(|_| {
            load_asset(path).map(|image| {
                // SAFETY: the image was just loaded on the main thread.
                let size = unsafe { image.size() };
                Size::new(size.width as f32, size.height as f32)
            })
        });
        self.image_sizes
            .lock()
            .expect("image size cache")
            .insert(path.clone(), size);
        size
    }
}
