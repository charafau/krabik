//! The `text` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::element::ElementKind;
use crate::engine::geometry::{Constraints, Size};
use crate::engine::measure::Measure;
use crate::style::{Color, ColorScheme};
use crate::style::{Font, FontWeight, IntoPoints, TextAlign};
use crate::widgets::IntrinsicSize;
use crate::widgets::Widget;

/// Platform-independent properties of a text node.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(name = Text, text, about = "A run of text.", new = "Creates text with the default size and color.")]
pub struct TextProps {
    /// The text to display.
    #[widget(arg)]
    pub content: String,
    /// Font size in logical points.
    #[widget(default = 16.0)]
    pub size: f32,
    /// Text color.
    #[widget(default = Color::TEXT)]
    pub color: Color,
    /// Font weight.
    #[widget(default = FontWeight::Normal)]
    pub weight: FontWeight,
    /// Aligns the text horizontally inside its frame.
    #[widget(default = TextAlign::Start, set(name = text_align))]
    pub align: TextAlign,
    /// Limits the number of lines the text may occupy; unlimited when `None`.
    #[widget(set(with = at_least_one_line, from = u32))]
    pub max_lines: Option<u32>,
    /// Truncates the last visible line with an ellipsis when the text does not fit.
    #[widget(set(flag = true))]
    pub ellipsis: bool,
    /// Draws the text in italics.
    #[widget(set(flag = true))]
    pub italic: bool,
    /// Draws a line under the text.
    #[widget(set(flag = true))]
    pub underline: bool,
    /// Draws a line through the text.
    #[widget(set(flag = true))]
    pub strikethrough: bool,
    /// Extra space between two characters in logical points; negative values
    /// tighten the text.
    #[widget(set(with = IntoPoints::into_points, from = impl IntoPoints))]
    pub letter_spacing: f32,
    /// The height of one line in logical points; the font's own when `None`.
    #[widget(set(with = line_height_points, from = impl IntoPoints))]
    pub line_height: Option<f32>,
    /// The font; the system's unless a file is named.
    #[widget(set)]
    pub font: Font,
    /// The parts of a text made with [`Text::rich`], each with a style of
    /// its own; empty for a text in one style. `content` is all of them
    /// joined.
    #[widget(colors)]
    pub spans: Vec<Span>,
    /// Whether the user can copy the text: by selecting any part of it on
    /// Android, by holding it and choosing "Copy" on iOS.
    #[widget(set(flag = true))]
    pub selectable: bool,
}

/// A part of a text with its own style, for [`Text::rich`].
///
/// A span only says what differs from the text it is in: anything that is
/// not set here comes from the [`Text`].
///
/// ```ignore
/// Text::rich([
///     Span::new("By continuing you accept the "),
///     Span::new("terms").bold().underline(),
///     Span::new("."),
/// ])
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    /// The characters of the span.
    pub text: String,
    /// Font size in logical points.
    pub size: Option<f32>,
    /// Text color.
    pub color: Option<Color>,
    /// Font weight.
    pub weight: Option<FontWeight>,
    /// Whether the span is drawn in italics.
    pub italic: Option<bool>,
    /// Whether a line is drawn under the span.
    pub underline: Option<bool>,
    /// Whether a line is drawn through the span.
    pub strikethrough: Option<bool>,
    /// The font of the span.
    pub font: Option<Font>,
}

impl Span {
    /// Creates a span that looks like the text around it.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: None,
            color: None,
            weight: None,
            italic: None,
            underline: None,
            strikethrough: None,
            font: None,
        }
    }

    /// Sets the font size in logical points.
    pub fn size(mut self, size: impl IntoPoints) -> Self {
        self.size = Some(size.into_points());
        self
    }

    /// Sets the text color: an ARGB number or a [`Color`].
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Renders the span in bold.
    pub fn bold(self) -> Self {
        self.weight(FontWeight::Bold)
    }

    /// Sets the font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Draws the span in italics.
    pub fn italic(mut self) -> Self {
        self.italic = Some(true);
        self
    }

    /// Draws a line under the span.
    pub fn underline(mut self) -> Self {
        self.underline = Some(true);
        self
    }

    /// Draws a line through the span.
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = Some(true);
        self
    }

    /// Draws the span in another font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Picks the value for `scheme` in the span's colour.
    pub(crate) fn resolve_colors(&mut self, scheme: ColorScheme) {
        if let Some(color) = &mut self.color {
            *color = color.resolve(scheme);
        }
    }
}

impl<S: Into<String>> From<S> for Span {
    fn from(text: S) -> Self {
        Self::new(text)
    }
}

impl<M> Text<M> {
    /// Creates a text from parts that each have a style of their own.
    ///
    /// The style set on the text itself applies to every span that does not
    /// say otherwise. A plain string is a span too:
    ///
    /// ```ignore
    /// Text::rich(["Total: ".into(), Span::new("12.50").bold()])
    /// ```
    pub fn rich(spans: impl IntoIterator<Item = Span>) -> Self {
        let spans: Vec<Span> = spans
            .into_iter()
            .filter(|span| !span.text.is_empty())
            .collect();
        let content: String = spans.iter().map(|span| span.text.as_str()).collect();

        let mut text = Self::new(content);
        if let ElementKind::Widget(Widget::Text(props)) = &mut text.0.kind {
            props.spans = spans;
        }
        text
    }
}

/// A span with everything it inherits filled in, placed in the text.
///
/// The offsets count UTF-16 code units, which is how both platforms index a
/// string.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextRun {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) size: f32,
    pub(crate) color: Color,
    pub(crate) weight: FontWeight,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) font: Font,
}

fn at_least_one_line(lines: u32) -> Option<u32> {
    Some(lines.max(1))
}

/// A line height has to be positive; anything else means the font's own.
fn line_height_points(height: impl IntoPoints) -> Option<f32> {
    let height = height.into_points();
    (height > 0.0).then_some(height)
}

/// Everything about a text that decides its size, as a key for the
/// platforms' measurement caches.
///
/// A property that changes the measured size and is missing here makes two
/// different texts share one cached size, and one of them is clipped.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TextMeasureKey {
    content: String,
    size: u32,
    weight: FontWeight,
    italic: bool,
    letter_spacing: u32,
    line_height: Option<u32>,
    max_lines: Option<u32>,
    ellipsis: bool,
    font: Font,
    /// Where each span is, with its size, weight, slant and font.
    runs: Vec<(u32, u32, u32, FontWeight, bool, Font)>,
}

impl TextProps {
    /// The key under which a measurement of this text may be cached.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) fn measure_key(&self) -> TextMeasureKey {
        TextMeasureKey {
            content: self.content.clone(),
            size: self.size.to_bits(),
            weight: self.weight,
            italic: self.italic,
            letter_spacing: self.letter_spacing.to_bits(),
            line_height: self.line_height.map(f32::to_bits),
            max_lines: self.max_lines,
            ellipsis: self.ellipsis,
            font: self.font.clone(),
            runs: self
                .runs()
                .iter()
                .map(|run| {
                    (
                        run.start,
                        run.end,
                        run.size.to_bits(),
                        run.weight,
                        run.italic,
                        run.font.clone(),
                    )
                })
                .collect(),
        }
    }

    /// The spans of a rich text, each with the style it ends up with; empty
    /// for a text in one style.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) fn runs(&self) -> Vec<TextRun> {
        let mut start = 0;
        self.spans
            .iter()
            .map(|span| {
                let end = start + span.text.encode_utf16().count() as u32;
                let run = TextRun {
                    start,
                    end,
                    size: span.size.unwrap_or(self.size),
                    color: span.color.unwrap_or(self.color),
                    weight: span.weight.unwrap_or(self.weight),
                    italic: span.italic.unwrap_or(self.italic),
                    underline: span.underline.unwrap_or(self.underline),
                    strikethrough: span.strikethrough.unwrap_or(self.strikethrough),
                    font: span.font.clone().unwrap_or_else(|| self.font.clone()),
                };
                start = end;
                run
            })
            .collect()
    }

    /// Whether the text needs more than a font, a size and a colour: a
    /// decoration, letter spacing, a line height or spans.
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    pub(crate) fn is_decorated(&self) -> bool {
        self.underline
            || self.strikethrough
            || self.letter_spacing != 0.0
            || self.line_height.is_some()
            || !self.spans.is_empty()
    }
}

impl IntrinsicSize for TextProps {
    /// Text is as large as the platform measures it at the available width.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        measurer.measure(
            self,
            Constraints {
                max_width,
                max_height: None,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{Element, Text};

    fn props(element: Element) -> TextProps {
        match element.into_raw().kind {
            ElementKind::Widget(Widget::Text(props)) => props,
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn a_plain_text_has_no_styling() {
        let text = props(Text::new("Hello").into());
        assert_eq!(text.weight, FontWeight::Normal);
        assert!(!text.italic && !text.underline && !text.strikethrough);
        assert_eq!(text.letter_spacing, 0.0);
        assert_eq!(text.line_height, None);
        assert!(!text.is_decorated());
    }

    #[test]
    fn the_builders_set_the_style() {
        let text = props(
            Text::new("Hello")
                .weight(FontWeight::Medium)
                .italic()
                .underline()
                .strikethrough()
                .letter_spacing(0.5)
                .line_height(24)
                .into(),
        );
        assert_eq!(text.weight, FontWeight::Medium);
        assert!(text.italic && text.underline && text.strikethrough);
        assert_eq!(text.letter_spacing, 0.5);
        assert_eq!(text.line_height, Some(24.0));
        assert!(text.is_decorated());
    }

    #[test]
    fn a_line_height_that_is_not_positive_is_ignored() {
        assert_eq!(
            props(Text::new("a").line_height(0).into()).line_height,
            None
        );
        assert_eq!(
            props(Text::new("a").line_height(-4.0).into()).line_height,
            None
        );
    }

    #[test]
    fn bold_is_weight_seven_hundred() {
        assert_eq!(props(Text::new("a").bold().into()).weight.value(), 700);
        assert!(FontWeight::Thin < FontWeight::Black);
        assert!(FontWeight::SemiBold.is_bold());
        assert!(!FontWeight::Medium.is_bold());
    }

    #[test]
    fn a_rich_text_joins_its_spans() {
        let text = props(
            Text::rich([
                Span::new("Total: "),
                Span::new(""),
                Span::new("12.50").bold(),
                " EUR".into(),
            ])
            .into(),
        );
        assert_eq!(text.content, "Total: 12.50 EUR");
        // the empty span is dropped
        assert_eq!(text.spans.len(), 3);
        assert!(text.is_decorated());
    }

    #[test]
    fn a_span_takes_what_it_does_not_set_from_the_text() {
        let accent = Color::fixed(0xFF00_897B);
        let text = props(
            Text::rich([
                Span::new("plain "),
                Span::new("styled")
                    .size(20)
                    .color(accent)
                    .bold()
                    .underline(),
            ])
            .size(14)
            .italic()
            .into(),
        );
        let runs = text.runs();

        assert_eq!((runs[0].start, runs[0].end), (0, 6));
        assert_eq!(runs[0].size, 14.0);
        assert_eq!(runs[0].color, Color::TEXT);
        assert_eq!(runs[0].weight, FontWeight::Normal);
        assert!(runs[0].italic && !runs[0].underline);

        assert_eq!((runs[1].start, runs[1].end), (6, 12));
        assert_eq!(runs[1].size, 20.0);
        assert_eq!(runs[1].color, accent);
        assert_eq!(runs[1].weight, FontWeight::Bold);
        assert!(runs[1].italic && runs[1].underline && !runs[1].strikethrough);
    }

    #[test]
    fn a_font_is_inherited_by_spans_that_name_none() {
        let display = Font::asset("fonts/Display.ttf");
        let mono = Font::asset("fonts/Mono.ttf");
        let text = props(
            Text::rich([Span::new("a"), Span::new("b").font(mono.clone())])
                .font(display.clone())
                .into(),
        );
        assert_eq!(text.font, display);
        let runs = text.runs();
        assert_eq!(runs[0].font, display);
        assert_eq!(runs[1].font, mono);
        assert_eq!(Font::default().asset_path(), None);
        assert_eq!(mono.asset_path(), Some("fonts/Mono.ttf"));
    }

    /// The platforms index strings in UTF-16 code units.
    #[test]
    fn runs_are_placed_in_utf16_units() {
        let text = props(Text::rich([Span::new("a\u{1F600}é"), Span::new("b")]).into());
        let runs = text.runs();
        // 'a' is one unit, the emoji two, 'é' one
        assert_eq!((runs[0].start, runs[0].end), (0, 4));
        assert_eq!((runs[1].start, runs[1].end), (4, 5));
    }

    #[test]
    fn a_span_colour_follows_the_appearance() {
        let mut text = props(
            Text::rich([Span::new("a").color(Color::adaptive(0xFF11_1111, 0xFFEE_EEEE))]).into(),
        );
        crate::widgets::WidgetProps::resolve_colors(&mut text, ColorScheme::Dark);
        assert_eq!(text.runs()[0].color.argb(), 0xFFEE_EEEE);
    }

    #[test]
    fn the_measure_key_tells_spans_apart() {
        let plain = props(Text::rich([Span::new("ab")]).into()).measure_key();
        let bold = props(Text::rich([Span::new("a").bold(), Span::new("b")]).into()).measure_key();
        let large =
            props(Text::rich([Span::new("a").size(30), Span::new("b")]).into()).measure_key();
        assert_ne!(plain, bold);
        assert_ne!(plain, large);
        assert_ne!(bold, large);
    }

    /// Every property that changes the size of a text has to change its key.
    #[test]
    fn the_measure_key_tells_styles_apart() {
        let plain = TextProps::new("Hello").measure_key();
        let variants: [fn(&mut TextProps); 8] = [
            |text| text.font = Font::asset("fonts/Display.ttf"),
            |text| text.size = 20.0,
            |text| text.weight = FontWeight::Medium,
            |text| text.italic = true,
            |text| text.letter_spacing = 1.0,
            |text| text.line_height = Some(30.0),
            |text| text.max_lines = Some(1),
            |text| text.ellipsis = true,
        ];
        for change in variants {
            let mut text = TextProps::new("Hello");
            change(&mut text);
            assert_ne!(text.measure_key(), plain);
        }

        // decorations and the colour do not change the size
        let mut text = TextProps::new("Hello");
        text.underline = true;
        text.strikethrough = true;
        assert_eq!(text.measure_key(), plain);
    }
}
