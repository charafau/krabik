//! Text and control measurement through off-screen Android views.

use crate::android::handles::get as jni_handles;
use crate::android::widgets::text::{apply_text_limits, apply_typeface};
use crate::android::{COMPLEX_UNIT_SP, MEASURE_AT_MOST, MEASURE_UNSPECIFIED, WRAP_CONTENT};
use crate::geometry::{Constraints, Size};
use crate::measure::{DEFAULT_SWITCH_HEIGHT, DEFAULT_SWITCH_WIDTH, Measure};
use crate::ui::{FontWeight, ImageProps, ImageSource, TextProps};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::refs::Global;
use jni::sys::jint;
use jni::{Env, JValue, JavaVM};
use std::collections::HashMap;
use std::sync::Mutex;

/// Measures text with an off-screen native `TextView`.
///
/// Results are cached because Taffy asks for the same intrinsic sizes several
/// times per layout pass.
pub(crate) struct AndroidMeasure {
    pub(crate) vm: JavaVM,
    pub(crate) context: Global<JObject<'static>>,
    pub(crate) text_view: Global<JObject<'static>>,
    pub(crate) density: f32,
    pub(crate) cache: Mutex<HashMap<MeasureKey, Size>>,
    pub(crate) switch_size: Mutex<Option<Size>>,
    pub(crate) image_sizes: Mutex<HashMap<String, Option<Size>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MeasureKey {
    pub(crate) content: String,
    pub(crate) size: u32,
    pub(crate) bold: bool,
    pub(crate) max_width: Option<u32>,
    pub(crate) max_lines: Option<u32>,
    pub(crate) ellipsis: bool,
}

impl AndroidMeasure {
    pub(crate) fn new(
        env: &mut Env<'_>,
        vm: &JavaVM,
        context: &JObject<'_>,
        density: f32,
    ) -> JniResult<Self> {
        let ids = jni_handles(env)?;
        let view = ids
            .text_view_ctor
            .construct(env, &ids.text_view, &[JValue::Object(context)])?;

        ids.set_include_font_padding
            .call_void(env, &view, &[JValue::Bool(false)])?;
        ids.set_padding.call_void(
            env,
            &view,
            &[
                JValue::Int(0),
                JValue::Int(0),
                JValue::Int(0),
                JValue::Int(0),
            ],
        )?;

        // `TextView.setText` consults the layout params while it decides
        // whether to request another layout pass, even for a detached view.
        let params = ids.layout_params_ctor.construct(
            env,
            &ids.layout_params,
            &[JValue::Int(WRAP_CONTENT), JValue::Int(WRAP_CONTENT)],
        )?;
        ids.set_layout_params
            .call_void(env, &view, &[JValue::Object(&params)])?;

        Ok(Self {
            vm: vm.clone(),
            context: jcall!(env, env.new_global_ref(context))?,
            text_view: jcall!(env, env.new_global_ref(&view))?,
            density,
            cache: Mutex::new(HashMap::new()),
            switch_size: Mutex::new(None),
            image_sizes: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn measure_with(
        &self,
        env: &mut Env<'_>,
        text: &TextProps,
        constraints: Constraints,
    ) -> JniResult<Size> {
        let view = self.text_view.as_obj();

        let string = jcall!(env, env.new_string(&text.content))?;
        let ids = jni_handles(env)?;
        ids.set_text
            .call_void(env, view, &[JValue::Object(string.as_ref())])?;
        ids.set_text_size.call_void(
            env,
            view,
            &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(text.size)],
        )?;
        let bold = matches!(text.weight, FontWeight::Bold);
        apply_typeface(env, view, bold)?;
        apply_text_limits(env, view, (text.max_lines, text.ellipsis))?;

        let width = match constraints.max_width {
            Some(max_width) => {
                let px = (max_width * self.density).ceil().max(0.0) as jint;
                measure_spec(env, px, MEASURE_AT_MOST)?
            }
            None => measure_spec(env, 0, MEASURE_UNSPECIFIED)?,
        };
        let height = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;

        let ids = jni_handles(env)?;
        ids.measure
            .call_void(env, view, &[JValue::Int(width), JValue::Int(height)])?;

        let measured_width = ids.measured_width.call(env, view, &[])?.i()?;
        let measured_height = ids.measured_height.call(env, view, &[])?.i()?;

        Ok(Size::new(
            measured_width as f32 / self.density,
            measured_height as f32 / self.density,
        ))
    }

    /// Measures a framework switch with wrap-content specs.
    pub(crate) fn measure_switch(&self) -> JniResult<Size> {
        self.vm.attach_current_thread(|env| {
            let ids = jni_handles(env)?;
            let view = ids.plain_switch_ctor.construct(
                env,
                &ids.plain_switch,
                &[JValue::Object(self.context.as_obj())],
            )?;
            let width = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;
            let height = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;
            ids.measure
                .call_void(env, &view, &[JValue::Int(width), JValue::Int(height)])?;
            let width = ids.measured_width.call(env, &view, &[])?.i()?;
            let height = ids.measured_height.call(env, &view, &[])?.i()?;
            Ok(Size::new(
                width as f32 / self.density,
                height as f32 / self.density,
            ))
        })
    }

    /// Reads the header dimensions of a bundled asset.
    pub(crate) fn measure_image(&self, path: &str) -> JniResult<Option<Size>> {
        self.vm.attach_current_thread(|env| {
            let ids = jni_handles(env)?;
            let source = jcall!(env, env.new_string(path))?;
            let packed = ids
                .image_size
                .call(
                    env,
                    &ids.krabik_images,
                    &[
                        JValue::Object(self.context.as_obj()),
                        JValue::Object(source.as_ref()),
                    ],
                )?
                .j()?;
            if packed == 0 {
                return Ok(None);
            }
            let packed = packed as u64;
            let width = (packed >> 32) as u32 as f32 / self.density;
            let height = (packed as u32) as f32 / self.density;
            Ok(Some(Size::new(width, height)))
        })
    }
}

impl Measure for AndroidMeasure {
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
        let key = MeasureKey {
            content: text.content.clone(),
            size: text.size.to_bits(),
            bold: matches!(text.weight, FontWeight::Bold),
            max_width: constraints.max_width.map(f32::to_bits),
            max_lines: text.max_lines,
            ellipsis: text.ellipsis,
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

        let measured = self
            .vm
            .attach_current_thread(|env| self.measure_with(env, text, constraints))
            .unwrap_or_else(|error| {
                eprintln!("text measurement failed: {error}");
                Size::ZERO
            });

        self.cache
            .lock()
            .expect("measurement cache")
            .insert(key, measured);
        measured
    }

    fn switch_size(&self) -> Size {
        if let Some(size) = *self.switch_size.lock().expect("switch size cache") {
            return size;
        }
        let size = self.measure_switch().unwrap_or_else(|error| {
            eprintln!("switch measurement failed: {error}");
            Size::new(DEFAULT_SWITCH_WIDTH, DEFAULT_SWITCH_HEIGHT)
        });
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
        let size = self.measure_image(path).unwrap_or_else(|error| {
            eprintln!("image measurement failed: {error}");
            None
        });
        self.image_sizes
            .lock()
            .expect("image size cache")
            .insert(path.clone(), size);
        size
    }
}

pub(crate) fn measure_spec(env: &mut Env<'_>, size: jint, mode: jint) -> JniResult<jint> {
    let ids = jni_handles(env)?;
    ids.make_measure_spec
        .call(
            env,
            &ids.measure_spec,
            &[JValue::Int(size), JValue::Int(mode)],
        )?
        .i()
}
