//! SVG assets on Android.
//!
//! `KrabikImages` reads an `.svg` asset and asks these two functions for
//! its size and for a PNG at the size of the view. They take bytes and
//! return bytes: no session is involved, so Kotlin calls the renderer from
//! a background thread.

use crate::android::run;
use crate::svg;
use jni::EnvUnowned;
use jni::objects::{JByteArray, JClass};
use jni::sys::{jboolean, jint, jlong};

/// The natural size of an SVG in logical points, packed as
/// `(width << 32) | height`, or zero when the bytes are not an SVG.
#[unsafe(export_name = "Java_dev_krabik_ui_KrabikImages_nativeSvgSize")]
pub extern "system" fn native_svg_size<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    bytes: JByteArray<'caller>,
) -> jlong {
    run(&mut unenv, "nativeSvgSize", |env| {
        let data = env.convert_byte_array(&bytes)?;
        Ok(svg::natural_size(&data).map_or(0, |size| {
            let width = size.width.round().max(0.0) as u32;
            let height = size.height.round().max(0.0) as u32;
            ((u64::from(width) << 32) | u64::from(height)) as jlong
        }))
    })
}

/// A PNG of the SVG at `width` by `height` pixels, or null when it cannot
/// be rendered.
#[unsafe(export_name = "Java_dev_krabik_ui_KrabikImages_nativeRenderSvg")]
pub extern "system" fn native_render_svg<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    bytes: JByteArray<'caller>,
    width: jint,
    height: jint,
    fill: jboolean,
) -> JByteArray<'caller> {
    run(&mut unenv, "nativeRenderSvg", |env| {
        let data = env.convert_byte_array(&bytes)?;
        let (width, height) = (width.max(0) as u32, height.max(0) as u32);
        match svg::render_png(&data, width, height, fill) {
            Some(png) => env.byte_array_from_slice(&png),
            None => Ok(JByteArray::default()),
        }
    })
}
