//! Shared geometry primitives.
//!
//! Layout runs in logical points (dp on Android, points on iOS). The platform
//! renderer converts the resulting rectangles to physical pixels.

/// A width and height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// The horizontal extent.
    pub width: f32,
    /// The vertical extent.
    pub height: f32,
}

impl Size {
    /// A zero-sized box.
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    /// Creates a size from its extents.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// A rectangle in the coordinate space of the root node.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// The left edge.
    pub x: f32,
    /// The top edge.
    pub y: f32,
    /// The horizontal extent.
    pub width: f32,
    /// The vertical extent.
    pub height: f32,
}

impl Rect {
    /// Creates a rectangle from its origin and extents.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Converts the frame to physical pixels for a native view.
    ///
    /// The origin is rounded to the nearest pixel; the extents are rounded and
    /// then clamped to zero, because a native view cannot have a negative size.
    pub fn to_pixels(self, density: f32) -> PixelFrame {
        PixelFrame {
            x: (self.x * density).round() as i32,
            y: (self.y * density).round() as i32,
            width: (self.width * density).round().max(0.0) as i32,
            height: (self.height * density).round().max(0.0) as i32,
        }
    }

    /// The right edge.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// The bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// The bounds a measured node has to fit into.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Constraints {
    /// The maximum width, when the parent has a definite width.
    pub max_width: Option<f32>,
    /// The maximum height, when the parent has a definite height.
    pub max_height: Option<f32>,
}

impl Constraints {
    /// No bounds in either axis.
    pub const UNBOUNDED: Self = Self {
        max_width: None,
        max_height: None,
    };

    /// Bounds the width.
    pub const fn width(max_width: f32) -> Self {
        Self {
            max_width: Some(max_width),
            max_height: None,
        }
    }
}

/// A frame converted to physical pixels, as a platform renderer needs it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelFrame {
    /// The left edge in pixels.
    pub x: i32,
    /// The top edge in pixels.
    pub y: i32,
    /// The horizontal extent in pixels.
    pub width: i32,
    /// The vertical extent in pixels.
    pub height: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_points_to_rounded_pixels() {
        let frame = Rect::new(10.4, 20.2, 100.0, 50.0).to_pixels(2.0);

        assert_eq!(
            frame,
            PixelFrame {
                x: 21,
                y: 40,
                width: 200,
                height: 100,
            }
        );
    }

    #[test]
    fn density_one_preserves_the_logical_frame() {
        let frame = Rect::new(3.0, 4.0, 5.0, 6.0).to_pixels(1.0);

        assert_eq!(
            frame,
            PixelFrame {
                x: 3,
                y: 4,
                width: 5,
                height: 6,
            }
        );
    }

    #[test]
    fn negative_origins_survive_but_sizes_are_clamped() {
        let frame = Rect::new(-0.8, -1.2, -10.0, -0.1).to_pixels(1.0);

        assert_eq!(frame.x, -1);
        assert_eq!(frame.y, -1);
        assert_eq!(frame.width, 0);
        assert_eq!(frame.height, 0);
    }

    #[test]
    fn fractional_density_rounds_to_the_nearest_pixel() {
        let frame = Rect::new(0.5, 0.4, 0.5, 0.9).to_pixels(3.0);

        assert_eq!(
            frame,
            PixelFrame {
                x: 2,
                y: 1,
                width: 2,
                height: 3,
            }
        );
    }
}
