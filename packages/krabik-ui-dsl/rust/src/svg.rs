//! SVG images, rendered in Rust so both platforms show the same pixels.
//!
//! The platforms hand over the bytes of an `.svg` asset and get back its
//! natural size or a PNG of it at the size it is shown at. A PNG is what
//! both `BitmapFactory` and `UIImage` take without further conversion.
//!
//! Everything static SVG has is supported, filters such as `feGaussianBlur`
//! included. Text is not: there are no fonts here to draw it with.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

use crate::geometry::Size;

/// The largest side a rendering may have, in pixels. A request beyond it is
/// almost certainly a layout that has not settled.
const MAX_SIDE: u32 = 4096;

/// Whether an asset is an SVG, going by its name.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
pub(crate) fn is_svg(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("svg"))
}

/// The size the document asks for, in logical points.
pub(crate) fn natural_size(data: &[u8]) -> Option<Size> {
    let size = Tree::from_data(data, &Options::default()).ok()?.size();
    Some(Size::new(size.width(), size.height()))
}

/// Renders the document into `width` by `height` pixels and encodes the
/// result as a PNG.
///
/// The drawing keeps its aspect ratio and is centred: it fits inside the
/// pixels, or with `fill` covers them and is cropped.
pub(crate) fn render_png(data: &[u8], width: u32, height: u32, fill: bool) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return None;
    }
    let tree = Tree::from_data(data, &Options::default()).ok()?;
    let size = tree.size();
    let (across, down) = (width as f32 / size.width(), height as f32 / size.height());
    let scale = if fill {
        across.max(down)
    } else {
        across.min(down)
    };
    let transform = Transform::from_scale(scale, scale).post_translate(
        (width as f32 - size.width() * scale) / 2.0,
        (height as f32 - size.height() * scale) / 2.0,
    );

    let mut pixmap = Pixmap::new(width, height)?;
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A red square on the left half of a 40 by 20 drawing.
    const HALF_RED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"
        viewBox="0 0 40 20"><rect width="20" height="20" fill="#ff0000"/></svg>"##;

    fn pixels(png: &[u8]) -> Pixmap {
        Pixmap::decode_png(png).expect("a png")
    }

    fn alpha(pixmap: &Pixmap, x: u32, y: u32) -> u8 {
        pixmap.pixel(x, y).expect("inside").alpha()
    }

    #[test]
    fn an_svg_is_known_by_its_name() {
        assert!(is_svg("logo.svg"));
        assert!(is_svg("art/Hero.SVG"));
        assert!(!is_svg("logo.png"));
        assert!(!is_svg("svg"));
    }

    #[test]
    fn the_natural_size_is_the_documents() {
        assert_eq!(
            natural_size(HALF_RED.as_bytes()),
            Some(Size::new(40.0, 20.0))
        );
        assert_eq!(natural_size(b"not an svg"), None);
    }

    #[test]
    fn a_drawing_fits_inside_the_pixels_it_is_given() {
        // Twice as large, in a square: letterboxed above and below.
        let png = render_png(HALF_RED.as_bytes(), 80, 80, false).expect("rendered");
        let pixmap = pixels(&png);
        assert_eq!((pixmap.width(), pixmap.height()), (80, 80));
        assert_eq!(alpha(&pixmap, 10, 5), 0, "above the drawing");
        assert_eq!(alpha(&pixmap, 10, 40), 255, "the red half");
        assert_eq!(alpha(&pixmap, 70, 40), 0, "the empty half");
    }

    #[test]
    fn a_filling_drawing_covers_the_pixels_and_is_cropped() {
        // Scaled to the height of the square, so only the middle shows.
        let png = render_png(HALF_RED.as_bytes(), 80, 80, true).expect("rendered");
        let pixmap = pixels(&png);
        assert_eq!(alpha(&pixmap, 10, 5), 255, "red reaches the top");
        assert_eq!(alpha(&pixmap, 70, 75), 0, "the empty half is on the right");
    }

    #[test]
    fn a_blur_filter_is_rendered() {
        let blurred = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40">
            <filter id="b" x="-50%" y="-50%" width="200%" height="200%">
                <feGaussianBlur stdDeviation="4"/>
            </filter>
            <rect x="10" y="10" width="20" height="20" fill="#0000ff" filter="url(#b)"/>
        </svg>"##;
        let pixmap = pixels(&render_png(blurred.as_bytes(), 40, 40, false).expect("rendered"));
        // Just outside the square there is colour that the blur spread.
        let edge = alpha(&pixmap, 7, 20);
        assert!(edge > 0 && edge < 255, "alpha {edge}");
    }

    #[test]
    fn nothing_is_rendered_for_an_empty_or_absurd_size() {
        assert!(render_png(HALF_RED.as_bytes(), 0, 10, false).is_none());
        assert!(render_png(HALF_RED.as_bytes(), MAX_SIDE + 1, 10, false).is_none());
        assert!(render_png(b"not an svg", 10, 10, false).is_none());
    }
}
