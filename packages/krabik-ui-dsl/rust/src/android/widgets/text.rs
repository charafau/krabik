//! The `text` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::android::widgets::Site;
use crate::android::{COMPLEX_UNIT_SP, GRAVITY_CENTER_HORIZONTAL, GRAVITY_END, GRAVITY_START};
use crate::tree::NodeKind;
use crate::ui::{Font, FontWeight, TextAlign, TextProps};
use crate::widgets::TextRun;
use crate::widgets::Widget;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue, jni_str};

/// How a string is drawn, apart from its size and colour.
///
/// `KrabikText.apply` takes these values; the bits of [`TextLook::flags`]
/// are repeated there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextLook<'a> {
    /// The font file, or `None` for the system font.
    pub(crate) font: Option<&'a str>,
    pub(crate) weight: FontWeight,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    /// Extra space between characters, in the unit of the text size.
    pub(crate) letter_spacing: f32,
    /// The height of a line, in the unit of the text size.
    pub(crate) line_height: Option<f32>,
}

const FLAG_ITALIC: jint = 1;
const FLAG_UNDERLINE: jint = 1 << 1;
const FLAG_STRIKETHROUGH: jint = 1 << 2;

/// How many numbers describe one run in the array `KrabikText.apply` takes.
const RUN_FIELDS: usize = 7;

/// The font index of a text or a run in the system font.
const SYSTEM_FONT: jint = -1;

/// The font files a text and its runs use, each once, and where a path is
/// in that list.
struct FontTable<'a>(Vec<&'a str>);

impl<'a> FontTable<'a> {
    fn index(&mut self, path: Option<&'a str>) -> jint {
        let Some(path) = path else {
            return SYSTEM_FONT;
        };
        let index = match self.0.iter().position(|known| *known == path) {
            Some(index) => index,
            None => {
                self.0.push(path);
                self.0.len() - 1
            }
        };
        index as jint
    }
}

fn flags(italic: bool, underline: bool, strikethrough: bool) -> jint {
    let mut flags = 0;
    if italic {
        flags |= FLAG_ITALIC;
    }
    if underline {
        flags |= FLAG_UNDERLINE;
    }
    if strikethrough {
        flags |= FLAG_STRIKETHROUGH;
    }
    flags
}

/// The runs of a rich text as numbers: for each run its start, its end, its
/// flags, its weight, its colour, the bits of its size and the index of its
/// font in `fonts`.
fn pack_runs<'a>(runs: &'a [TextRun], fonts: &mut FontTable<'a>) -> Vec<jint> {
    let mut packed = Vec::with_capacity(runs.len() * RUN_FIELDS);
    for run in runs {
        packed.extend([
            run.start as jint,
            run.end as jint,
            flags(run.italic, run.underline, run.strikethrough),
            jint::from(run.weight.value()),
            run.color.argb() as jint,
            run.size.to_bits() as jint,
            fonts.index(run.font.asset_path()),
        ]);
    }
    packed
}

impl<'a> TextLook<'a> {
    /// A label that has a font and a weight and nothing else, as a button's.
    pub(crate) fn plain(font: &'a Font, weight: FontWeight) -> Self {
        Self {
            font: font.asset_path(),
            weight,
            italic: false,
            underline: false,
            strikethrough: false,
            letter_spacing: 0.0,
            line_height: None,
        }
    }

    /// The look of a text widget.
    pub(crate) fn of(props: &'a TextProps) -> Self {
        Self {
            font: props.font.asset_path(),
            weight: props.weight,
            italic: props.italic,
            underline: props.underline,
            strikethrough: props.strikethrough,
            letter_spacing: props.letter_spacing,
            line_height: props.line_height,
        }
    }

    fn flags(&self) -> jint {
        flags(self.italic, self.underline, self.strikethrough)
    }
}

/// Sets the string of a text view and how it is drawn.
///
/// The text size has to be set before, because letter spacing is relative
/// to it. Rendering and measuring both go through here, so that a frame is
/// measured for exactly what is drawn in it.
///
/// `runs` are the spans of a rich text; with them, each run brings its own
/// font, colour and decorations, and `look` only gives the spacing.
pub(crate) fn apply_text(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    content: &str,
    look: TextLook<'_>,
    runs: &[TextRun],
) -> JniResult<()> {
    let string = jcall!(env, env.new_string(content))?;
    let mut fonts = FontTable(Vec::new());
    let font = fonts.index(look.font);
    let packed = pack_runs(runs, &mut fonts);
    let font_array = if fonts.0.is_empty() {
        JObject::null()
    } else {
        let array = env.new_object_array(
            fonts.0.len() as jint,
            jni_str!("java/lang/String"),
            JObject::null(),
        )?;
        for (index, path) in fonts.0.iter().enumerate() {
            let path = jcall!(env, env.new_string(path))?;
            let object: &JObject<'_> = path.as_ref();
            array.set_element(env, index, object)?;
        }
        JObject::from(array)
    };
    let run_array = if packed.is_empty() {
        JObject::null()
    } else {
        let array = env.new_int_array(packed.len())?;
        array.set_region(env, 0, &packed)?;
        JObject::from(array)
    };
    let ids = jni_handles(env)?;
    ids.apply_text.call_void(
        env,
        &ids.krabik_text,
        &[
            JValue::Object(view),
            JValue::Object(string.as_ref()),
            JValue::Int(jint::from(look.weight.value())),
            JValue::Int(look.flags()),
            JValue::Float(look.letter_spacing),
            JValue::Float(look.line_height.unwrap_or(0.0)),
            JValue::Object(&run_array),
            JValue::Object(&font_array),
            JValue::Int(font),
        ],
    )
}

pub(crate) fn update_text(env: &mut Env<'_>, view: &JObject<'_>, kind: &NodeKind) -> JniResult<()> {
    let Some((content, size, color, look)) = text_properties(kind) else {
        return Ok(());
    };

    // The off-screen view used for measurement disables font padding. Do the
    // same here so the rendered metrics match the measured ones, otherwise the
    // extra padding pushes descenders out of the frame.
    let ids = jni_handles(env)?;
    ids.set_include_font_padding
        .call_void(env, view, &[JValue::Bool(false)])?;

    ids.set_text_size.call_void(
        env,
        view,
        &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(size)],
    )?;
    let runs = match kind {
        NodeKind::Widget(Widget::Text(props)) => props.runs(),
        _ => Vec::new(),
    };
    apply_text(env, view, content, look, &runs)?;
    ids.set_text_color
        .call_void(env, view, &[JValue::Int(color as jint)])?;

    apply_text_limits(env, view, text_limits(kind))?;

    if let NodeKind::Widget(Widget::Text(props)) = kind {
        let gravity = match props.align {
            TextAlign::Start => GRAVITY_START,
            TextAlign::Center => GRAVITY_CENTER_HORIZONTAL,
            TextAlign::End => GRAVITY_END,
        };
        ids.set_gravity
            .call_void(env, view, &[JValue::Int(gravity)])?;
    }

    Ok(())
}

/// The line limit and truncation of a text-bearing node.
pub(crate) fn text_limits(kind: &NodeKind) -> (Option<u32>, bool) {
    match kind {
        NodeKind::Widget(Widget::Text(props)) => (props.max_lines, props.ellipsis),
        _ => (None, false),
    }
}

pub(crate) fn apply_text_limits(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    limits: (Option<u32>, bool),
) -> JniResult<()> {
    let (max_lines, ellipsis) = limits;
    let max_lines = max_lines.map_or(jint::MAX, |lines| {
        lines.max(1).min(jint::MAX as u32) as jint
    });
    let ids = jni_handles(env)?;
    ids.set_max_lines
        .call_void(env, view, &[JValue::Int(max_lines)])?;

    let truncate_at = if ellipsis {
        ids.truncate_at_end.get(env, &ids.truncate_at)?.l()?
    } else {
        JObject::null()
    };

    ids.set_ellipsize
        .call_void(env, view, &[JValue::Object(&truncate_at)])?;
    Ok(())
}

/// Sets the font of a control's label, which has a weight and nothing else.
pub(crate) fn apply_typeface(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    weight: FontWeight,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.apply_typeface.call_void(
        env,
        &ids.krabik_text,
        &[
            JValue::Object(view),
            JValue::Int(jint::from(weight.value())),
            JValue::Bool(false),
        ],
    )
}

pub(crate) fn text_properties(kind: &NodeKind) -> Option<(&str, f32, u32, TextLook<'_>)> {
    match kind {
        NodeKind::Widget(Widget::Text(props)) => Some((
            &props.content,
            props.size,
            props.color.argb(),
            TextLook::of(props),
        )),
        NodeKind::Widget(Widget::Button(props)) => Some((
            &props.label,
            props.size,
            props.color.argb(),
            TextLook::plain(&props.font, props.weight),
        )),
        _ => None,
    }
}

/// Creates the native view of a text.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    site.stock(env, &ids.text_view, &ids.text_view_ctor)
}
