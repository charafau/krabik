//! Intrinsic measurement of content-dependent nodes.
//!
//! Taffy cannot know how large a native widget wants to be. Text is the most
//! important case: its intrinsic size depends on the font and on the available
//! width. Platforms implement [`Measure`] with their native text APIs; the
//! approximate implementation in this module keeps the tree testable on the
//! host and acts as a fallback.

use crate::geometry::{Constraints, Size};
use crate::ui::TextProps;

/// Measures content that the layout engine cannot size by itself.
pub trait Measure: Send + Sync {
    /// Returns the intrinsic size of the text under the given constraints.
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size;
}

/// A cheap text measurement that does not need a platform.
///
/// It assumes an average glyph width of `0.55 * font size` and a line height of
/// `1.25 * font size`. Real platforms must replace it with native measurement.
#[derive(Clone, Copy, Debug, Default)]
pub struct ApproximateMeasure;

impl Measure for ApproximateMeasure {
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
        if text.content.is_empty() {
            return Size::ZERO;
        }

        let glyph_width = text.size * 0.55;
        let line_height = text.size * 1.25;
        let natural_width = text.content.chars().count() as f32 * glyph_width;

        let width = constraints
            .max_width
            .map_or(natural_width, |max| natural_width.min(max));
        let lines = if width > 0.0 {
            (natural_width / width).ceil().max(1.0)
        } else {
            1.0
        };
        let lines = text
            .max_lines
            .map_or(lines, |max| lines.min(max.max(1) as f32));

        Size::new(width, lines * line_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(content: &str) -> TextProps {
        TextProps::new(content)
    }

    #[test]
    fn empty_text_has_no_size() {
        assert_eq!(
            ApproximateMeasure.measure(&text(""), Constraints::UNBOUNDED),
            Size::ZERO
        );
    }

    #[test]
    fn text_is_measured_from_its_glyphs() {
        // 5 glyphs * (16.0 * 0.55) = 44 wide, one line of 20 high
        assert_eq!(
            ApproximateMeasure.measure(&text("Hello"), Constraints::UNBOUNDED),
            Size::new(44.0, 20.0)
        );
    }

    #[test]
    fn available_width_wraps_the_text() {
        let constraints = Constraints::width(22.0);
        let measured = ApproximateMeasure.measure(&text("Hello"), constraints);

        assert_eq!(measured.width, 22.0);
        assert_eq!(measured.height, 40.0);
    }

    #[test]
    fn constraints_do_not_stretch_short_text() {
        let measured = ApproximateMeasure.measure(&text("Hi"), Constraints::width(500.0));
        assert_eq!(measured.width, 17.6);
    }

    #[test]
    fn max_lines_clamps_the_measured_height() {
        let constraints = Constraints::width(22.0);

        // "Hello" wraps onto two lines at 22dp, but the limit is one.
        let mut limited = text("Hello");
        limited.max_lines = Some(1);
        assert_eq!(
            ApproximateMeasure.measure(&limited, constraints),
            Size::new(22.0, 20.0)
        );

        // A limit above the required line count changes nothing.
        let mut generous = text("Hello");
        generous.max_lines = Some(5);
        assert_eq!(
            ApproximateMeasure.measure(&generous, constraints),
            Size::new(22.0, 40.0)
        );
    }
}
