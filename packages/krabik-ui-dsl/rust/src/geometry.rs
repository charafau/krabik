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
