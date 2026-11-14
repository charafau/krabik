//! Packs node frames into the parallel arrays a platform batch API consumes.
//!
//! Renderers that apply a frame with one call per node pay a JNI (or FFI)
//! crossing per node. Batching trades that for a registry keyed by node id and
//! three primitive arrays, which cross the boundary in constant time.
//!
//! The packed layout is one id per node, four integers per node in
//! `x, y, width, height` order, and six integers per node describing its
//! animation.

use crate::geometry::Rect;
use crate::tree::NodeId;
use crate::ui::Animation;

/// Node ids and their frames, ready to hand to a batch API.
#[derive(Clone, Debug, Default)]
pub struct FrameBatch {
    ids: Vec<i64>,
    frames: Vec<i32>,
    animations: Vec<i32>,
}

impl FrameBatch {
    /// An empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one node frame, converting the logical points to pixels.
    ///
    /// The animation is packed as a duration and five curve integers. A
    /// missing or zero-duration animation tells the platform to apply the
    /// frame directly.
    pub fn push(&mut self, id: NodeId, frame: Rect, density: f32, animation: Option<&Animation>) {
        let pixels = frame.to_pixels(density);
        self.ids.push(id.0 as i64);
        self.frames
            .extend([pixels.x, pixels.y, pixels.width, pixels.height]);
        self.animations.push(animation.map_or(0, |animation| {
            animation.duration_ms.min(i32::MAX as u32) as i32
        }));
        self.animations
            .extend(animation.map_or([0; 5], |animation| animation.curve.packed()));
    }

    /// Whether any frame was added.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// The number of frames in the batch.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// The node ids, in the order they were added.
    pub fn ids(&self) -> &[i64] {
        &self.ids
    }

    /// The frames, four integers per node in `x, y, width, height` order.
    pub fn frames(&self) -> &[i32] {
        &self.frames
    }

    /// The animations, six integers per node in `duration, curve kind, params`
    /// order.
    pub fn animations(&self) -> &[i32] {
        &self.animations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_batch_has_no_arrays() {
        let batch = FrameBatch::new();

        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert!(batch.ids().is_empty());
        assert!(batch.frames().is_empty());
    }

    #[test]
    fn frames_are_packed_four_integers_per_node() {
        let mut batch = FrameBatch::new();
        batch.push(NodeId(7), Rect::new(1.0, 2.0, 3.0, 4.0), 1.0, None);

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.ids(), &[7]);
        assert_eq!(batch.frames(), &[1, 2, 3, 4]);
    }

    #[test]
    fn order_is_preserved_across_nodes() {
        let mut batch = FrameBatch::new();
        batch.push(NodeId(3), Rect::new(0.0, 0.0, 10.0, 10.0), 1.0, None);
        batch.push(NodeId(1), Rect::new(0.0, 10.0, 10.0, 10.0), 1.0, None);
        batch.push(NodeId(2), Rect::new(0.0, 20.0, 10.0, 10.0), 1.0, None);

        assert_eq!(batch.ids(), &[3, 1, 2]);
        assert_eq!(batch.frames().len(), 12);
        assert_eq!(batch.frames()[4..8], [0, 10, 10, 10]);
    }

    #[test]
    fn animations_are_packed_six_integers_per_node() {
        let mut batch = FrameBatch::new();
        batch.push(
            NodeId(1),
            Rect::new(0.0, 0.0, 10.0, 10.0),
            1.0,
            Some(&Animation::new(300).curve(crate::ui::AnimationCurve::EaseIn)),
        );
        batch.push(NodeId(2), Rect::new(0.0, 0.0, 10.0, 10.0), 1.0, None);

        assert_eq!(batch.animations(), &[300, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn custom_curves_use_fixed_point_parameters() {
        let mut batch = FrameBatch::new();
        let animation =
            Animation::new(400).curve(crate::ui::AnimationCurve::cubic_bezier(0.1, 0.2, 0.3, 0.9));
        batch.push(
            NodeId(1),
            Rect::new(0.0, 0.0, 10.0, 10.0),
            1.0,
            Some(&animation),
        );

        assert_eq!(batch.animations(), &[400, 1, 1_000, 2_000, 3_000, 9_000]);
    }

    #[test]
    fn density_is_applied_to_every_frame() {
        let mut batch = FrameBatch::new();
        batch.push(NodeId(1), Rect::new(1.0, 2.0, 3.0, 4.0), 2.0, None);

        assert_eq!(batch.frames(), &[2, 4, 6, 8]);
    }
}
