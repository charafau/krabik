//! The UI runtime.
//!
//! The runtime owns the current tree and its frames. Every update rebuilds the
//! tree, reconciles it with the previous one, lays it out, and emits one batch
//! of operations for the platform renderer.

use std::collections::HashMap;

use crate::geometry::{Rect, Size};
use crate::layout::{self, LayoutResult};
use crate::measure::Measure;
use crate::render::{self, Op, Renderer};
use crate::tree::{self, BuildInfo, ListExtent, ListMeasurements, NodeId, ROOT_ID, UiNode};
use crate::ui::{ListProps, RawElement};

/// The result of rebuilding the UI: the used size and the operations to apply.
#[derive(Clone, Debug, Default)]
pub struct Update {
    /// The size the root node occupies in the viewport.
    pub size: Size,
    /// The operations the renderer has to apply.
    pub ops: Vec<Op>,
}

impl Update {
    /// Whether the renderer has nothing to do.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// The row that currently holds focus inside a recycled list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowFocus {
    /// The recycled list that owns the row.
    pub list: NodeId,
    /// The item index the focused row shows.
    pub index: usize,
    /// The field node inside the row that has focus.
    pub field: NodeId,
    /// Whether the row was torn down while the field held focus.
    ///
    /// A blur in this state is a consequence of recycling, not a user action,
    /// so the runtime keeps the focus to restore it when the row comes back.
    pub parked: bool,
}

/// Owns the UI tree and coordinates layout and reconciliation.
pub struct Runtime {
    tree: UiNode,
    layout: LayoutResult,
    measurer: Box<dyn Measure>,
    /// Scroll offsets in logical points, keyed by the scroll node id.
    scroll_offsets: HashMap<NodeId, f32>,
    /// The extent of every list in the last built tree.
    lists: HashMap<NodeId, ListExtent>,
    /// Measured heights and prefix sums for variable-height lists.
    measurements: HashMap<NodeId, ListMeasurements>,
    /// The node that currently has native focus, if any.
    focused: Option<NodeId>,
    /// For each list, the item index that holds the focused node.
    focused_rows: HashMap<NodeId, usize>,
    /// Focus inside a recycled list, kept across the row's recycling.
    row_focus: Option<RowFocus>,
    /// Item sources of recycled lists, used to build a row on demand.
    row_sources: HashMap<NodeId, ListProps>,
    /// The viewport of the last update, needed to window lists on scroll.
    viewport: Size,
    /// Incremented on every rebuild; stamped into recycled lists so their
    /// adapters rebind the visible rows after an app state change.
    revision: u64,
}

impl Runtime {
    /// Creates a runtime with the given measurement implementation.
    pub fn new(measurer: Box<dyn Measure>) -> Self {
        Self {
            tree: UiNode::empty(),
            layout: LayoutResult::default(),
            measurer,
            scroll_offsets: HashMap::new(),
            lists: HashMap::new(),
            measurements: HashMap::new(),
            focused: None,
            focused_rows: HashMap::new(),
            row_focus: None,
            row_sources: HashMap::new(),
            viewport: Size::ZERO,
            revision: 0,
        }
    }

    /// Rebuilds the tree from `element` and returns the batch to apply.
    pub fn update(&mut self, element: &RawElement, viewport: Size) -> Update {
        // A viewport change relayouts every node; animating that would make a
        // rotation or split-screen resize look like a deliberate transition.
        let suppress_animation = self.viewport != viewport;
        self.viewport = viewport;

        // Variable-height lists converge in a couple of passes: the first one
        // measures the rows it materialized, the next uses those heights to
        // recompute the prefix sums and the window. Cap the loop so a row whose
        // height oscillates with the window cannot spin forever.
        let mut iteration = 0;
        let (mut next, info, layout) = loop {
            let (next, info) = tree::build_with_focus(
                element,
                &self.scroll_offsets,
                viewport,
                &mut self.measurements,
                &self.focused_rows,
                self.focused,
            );
            for id in &info.reset_scroll {
                self.scroll_offsets.remove(id);
            }
            let layout = layout::compute(&next, viewport, self.measurer.as_ref());
            let changed = self.record_heights(&info, &layout);
            iteration += 1;
            if !changed || iteration >= 3 {
                break (next, info, layout);
            }
        };

        // RecyclerView does not know that the application description changed;
        // a new revision is part of the node properties, so the diff emits an
        // update and the adapter rebinds its attached cells.
        self.revision = self.revision.wrapping_add(1);
        tree::stamp_recycler_revision(&mut next, self.revision);

        // A focused node that disappeared (for example a screen swap) cannot
        // be pinned anymore.
        let focus_lost = self
            .focused
            .is_some_and(|id| tree::find(&next, id).is_none());
        if focus_lost {
            self.focused = None;
            self.focused_rows.clear();
        }

        if let Some(focus) = self.row_focus {
            // A screen switch or a shorter list invalidates the logical focus.
            let present = info
                .recycled
                .get(&focus.list)
                .is_some_and(|props| focus.index < props.item_count);
            if !present {
                self.row_focus = None;
            }
        }

        self.lists = info.lists;
        self.row_sources = info.recycled;
        self.focused_rows = if focus_lost {
            HashMap::new()
        } else {
            info.focused_rows
        };

        let mut ops = render::diff(&self.tree, &next);
        let mut layout_ops = render::diff_layout(&self.tree, &next, &self.layout, &layout);
        if suppress_animation {
            for op in &mut layout_ops {
                if let render::Op::Layout { animation, .. } = op {
                    *animation = None;
                }
            }
        }
        ops.extend(layout_ops);

        let size = layout
            .frames
            .get(&ROOT_ID)
            .map_or(viewport, |root| Size::new(root.width, root.height));

        self.tree = next;
        self.layout = layout;

        Update { size, ops }
    }

    /// Stores the heights the last layout produced for variable-height rows.
    ///
    /// Returns whether anything changed, which asks for another build so the
    /// prefix sums and the materialized window follow the new measurements.
    fn record_heights(&mut self, info: &BuildInfo, layout: &LayoutResult) -> bool {
        let mut changed = false;

        for (list_id, rows) in &info.rows {
            let Some(measurement) = self.measurements.get_mut(list_id) else {
                continue;
            };

            for (row_id, key_hash, index) in rows {
                let Some(frame) = layout.frames.get(row_id) else {
                    continue;
                };
                let height = frame.height.max(0.0);
                let previous = measurement
                    .heights
                    .get(key_hash)
                    .copied()
                    .unwrap_or(measurement.estimate);
                if (previous - height).abs() > 0.5 {
                    measurement.heights.insert(*key_hash, height);
                    // Patch the cached prefix in place: every following item
                    // shifts by the height delta. A full rebuild would be
                    // O(n) per measured row.
                    let start = index.saturating_add(1);
                    if let Some(suffix) = measurement.prefix.get_mut(start..) {
                        let delta = height - previous;
                        for value in suffix {
                            *value += delta;
                        }
                    }
                    changed = true;
                }
            }
        }

        changed
    }

    /// Records the focus of a native input.
    ///
    /// The runtime keeps the row that holds the focused node materialized, so
    /// scrolling does not tear the field down and lose its focus and text.
    /// Returns whether the focused node changed.
    pub fn set_focus(&mut self, id: NodeId, focused: bool) -> bool {
        if focused {
            if self.focused == Some(id) {
                return false;
            }
            self.focused = Some(id);
            self.focused_rows.clear();
            // A main-tree input takes focus away from a recycled row.
            self.row_focus = None;
            true
        } else if self.focused == Some(id) {
            self.focused = None;
            self.focused_rows.clear();
            true
        } else {
            false
        }
    }

    /// Records focus of a field inside a recycled row.
    ///
    /// Recycled rows are not part of the main tree, so their focus is tracked
    /// separately and survives the row being recycled. A blur while the row is
    /// parked is not treated as a user action.
    pub fn set_row_focus(
        &mut self,
        list: NodeId,
        index: usize,
        field: NodeId,
        focused: bool,
    ) -> bool {
        if focused {
            let next = RowFocus {
                list,
                index,
                field,
                parked: false,
            };
            let changed = self.row_focus != Some(next);
            self.row_focus = Some(next);
            self.focused = None;
            self.focused_rows.clear();
            changed
        } else if self.row_focus.is_some_and(|focus| focus.field == field) {
            if self.row_focus.is_some_and(|focus| focus.parked) {
                false
            } else {
                self.row_focus = None;
                true
            }
        } else {
            false
        }
    }

    /// Marks the focused row as torn down before its field loses focus.
    pub fn park_row_focus(&mut self, field: NodeId) -> bool {
        if let Some(focus) = &mut self.row_focus
            && focus.field == field
            && !focus.parked
        {
            focus.parked = true;
            return true;
        }
        false
    }

    /// The row focus, for a renderer that is about to rebind a row.
    pub fn row_focus(&self) -> Option<RowFocus> {
        self.row_focus
    }

    /// Records the scroll offset of a list and reports whether its window
    /// changed and the UI has to be rebuilt.
    ///
    /// Sub-pixel and sub-row scrolls only update the stored offset: the
    /// materialized rows stay valid until the visible range actually moves.
    pub fn scroll(&mut self, id: NodeId, offset: f32) -> bool {
        let Some(extent) = self.lists.get(&id).copied() else {
            return false;
        };

        let Some(height) = extent.item_height else {
            let Some(measurement) = self.measurements.get_mut(&id) else {
                return false;
            };
            let total = measurement.prefix.last().copied().unwrap_or(0.0);
            let offset = offset.clamp(0.0, (total - self.viewport.height).max(0.0));
            let previous = self.scroll_offsets.insert(id, offset).unwrap_or(0.0);
            if (previous - offset).abs() < f32::EPSILON {
                return false;
            }

            let pin = self.focused_rows.get(&id).copied();
            let before = measurement.window;
            let after = tree::variable_window(
                &measurement.prefix,
                self.viewport.height,
                offset,
                extent.buffer,
                pin,
            );
            return before != after;
        };

        let offset = offset.clamp(
            0.0,
            (extent.count as f32 * height - self.viewport.height).max(0.0),
        );
        let previous = self.scroll_offsets.insert(id, offset).unwrap_or(0.0);
        if (previous - offset).abs() < f32::EPSILON {
            return false;
        }

        let pin = self.focused_rows.get(&id).copied();
        let before = tree::fixed_window(extent, self.viewport.height, previous, pin);
        let after = tree::fixed_window(extent, self.viewport.height, offset, pin);
        before != after
    }

    /// The item source of a recycled list, for building rows on demand.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) fn row_source(&self, id: NodeId) -> Option<&ListProps> {
        self.row_sources.get(&id)
    }

    /// The number of items a recycled list currently reports.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) fn recycled_item_count(&self, id: NodeId) -> Option<usize> {
        self.row_sources.get(&id).map(|props| props.item_count)
    }

    /// Builds and lays out one recycled row with its own root id.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) fn layout_subtree(
        &self,
        element: &RawElement,
        root: NodeId,
        viewport: Size,
    ) -> (UiNode, LayoutResult) {
        let (tree, _) = tree::build_at(element, root);
        let layout = layout::compute(&tree, viewport, self.measurer.as_ref());
        (tree, layout)
    }

    /// The scroll offset recorded for a node, if any.
    pub fn scroll_offset(&self, id: NodeId) -> f32 {
        self.scroll_offsets.get(&id).copied().unwrap_or(0.0)
    }

    /// Rebuilds the tree and applies the resulting batch in one call.
    pub fn render(
        &mut self,
        element: &RawElement,
        viewport: Size,
        renderer: &mut dyn Renderer,
    ) -> Size {
        let update = self.update(element, viewport);
        renderer.apply(&update.ops);
        update.size
    }

    /// The current tree.
    pub fn tree(&self) -> &UiNode {
        &self.tree
    }

    /// The frames of the last layout pass.
    pub fn frames(&self) -> &HashMap<NodeId, Rect> {
        &self.layout.frames
    }

    /// The height the content needs, regardless of the viewport's height:
    /// the bottom edge of the lowest top-level node.
    ///
    /// The synthetic root always fills the viewport, so a host that should
    /// wrap its content, such as a bottom sheet, asks for this instead.
    pub fn content_height(&self) -> f32 {
        self.tree
            .children
            .iter()
            .filter_map(|child| self.layout.frames.get(&child.id))
            .map(|frame| frame.y + frame.height)
            .fold(0.0, f32::max)
    }

    /// The scrollable content sizes of the last layout pass.
    pub fn content_sizes(&self) -> &HashMap<NodeId, Size> {
        &self.layout.content_sizes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Constraints;
    use crate::host::Actions;
    use crate::measure::ApproximateMeasure;
    use crate::render::RecordingRenderer;
    use crate::tree::NodeKind;
    use crate::ui::Element;
    use crate::widgets::Widget;
    use std::sync::Arc;

    use crate::ui::{Button, Column, Length, List, Switch, Text, TextField, TextProps};

    /// What the widget `id` asks to be told about.
    fn actions(runtime: &Runtime, id: NodeId) -> Actions {
        tree::find(runtime.tree(), id)
            .and_then(Actions::of)
            .unwrap_or_default()
    }

    fn viewport() -> Size {
        Size::new(300.0, 600.0)
    }

    fn screen(content: &str) -> Element {
        Column::new()
            .child(Text::new(content))
            .child(Button::new("Click").on_click(()))
            .into()
    }

    /// Layout operations are applied after every other operation of a batch.
    fn assert_layout_suffix(ops: &[Op]) {
        if let Some(first) = ops.iter().position(|op| matches!(op, Op::Layout { .. })) {
            assert!(
                ops[first..]
                    .iter()
                    .all(|op| matches!(op, Op::Layout { .. })),
                "layout operations must come last: {ops:?}"
            );
        }
    }

    #[test]
    fn content_height_ignores_the_viewport_height() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let element: Element = Column::new()
            .padding(10)
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();
        runtime.update(&element, Size::new(320.0, 900.0));
        let height = runtime.content_height();
        assert!(height > 0.0 && height < 900.0, "content height {height}");
    }

    #[test]
    fn layout_operations_come_last_in_every_batch() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));

        let update = runtime.update(&screen("Hello"), viewport());
        assert!(
            update.ops.iter().any(|op| matches!(op, Op::Create(_))),
            "{:?}",
            update.ops
        );
        assert_layout_suffix(&update.ops);

        // An update and a layout change in the same frame.
        let update = runtime.update(&screen("Hello, Rust!"), Size::new(200.0, 500.0));
        assert!(
            update.ops.iter().any(|op| matches!(op, Op::Update(_))),
            "{:?}",
            update.ops
        );
        assert_layout_suffix(&update.ops);

        // Viewport-only changes emit layouts and nothing else.
        let update = runtime.update(&screen("Hello, Rust!"), Size::new(180.0, 400.0));
        assert!(!update.ops.is_empty(), "{:?}", update.ops);
        assert!(
            update.ops.iter().all(|op| matches!(op, Op::Layout { .. })),
            "{:?}",
            update.ops
        );
    }

    #[test]
    fn the_first_update_creates_every_native_node() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let update = runtime.update(&screen("Hello"), viewport());

        let creates = update
            .ops
            .iter()
            .filter(|op| matches!(op, Op::Create(_)))
            .count();
        assert_eq!(creates, 2);
        assert_eq!(
            update
                .ops
                .iter()
                .filter(|op| matches!(op, Op::Layout { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn rebuilding_an_unchanged_screen_emits_nothing() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());
        let update = runtime.update(&screen("Hello"), viewport());

        assert!(update.is_empty(), "{:?}", update.ops);
    }

    #[test]
    fn changing_text_emits_a_single_update() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());
        let update = runtime.update(&screen("Hello, Rust!"), viewport());

        assert_eq!(update.ops.len(), 1, "{:?}", update.ops);
        assert!(matches!(&update.ops[0], Op::Update(_)));
    }

    #[test]
    fn buttons_report_clicks() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());

        let button = runtime.tree().children[0]
            .children
            .iter()
            .find(|node| matches!(node.kind, NodeKind::Widget(Widget::Button(_))))
            .unwrap();

        assert!(actions(&runtime, button.id).click.is_set());
        assert_eq!(actions(&runtime, ROOT_ID), Actions::default());
    }

    #[test]
    fn text_fields_report_changes_only() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let element: Element = Column::new()
            .child(TextField::new("").placeholder("Name").on_change(|_| ()))
            .into();
        runtime.update(&element, viewport());

        let field = runtime.tree().children[0].children[0].id;
        assert!(actions(&runtime, field).text.is_set());
        assert!(!actions(&runtime, field).click.is_set());
    }

    #[test]
    fn switches_report_toggles() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let element: Element = Column::new()
            .child(Switch::new(false).on_toggle(|_| ()))
            .into();
        runtime.update(&element, viewport());

        let switch = runtime.tree().children[0].children[0].id;
        assert!(actions(&runtime, switch).toggle.is_set());
    }

    #[test]
    fn a_handler_returns_the_message_it_was_given() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        // The message is an ordinary value; here a string built at runtime.
        let element: Element<String> = Column::new()
            .child(Button::new("Delete").on_click(format!("note:delete:{}", 2)))
            .into();
        runtime.update(&element, viewport());

        let button = runtime.tree().children[0].children[0].id;
        let message = actions(&runtime, button)
            .click
            .message(())
            .expect("the button reports clicks");
        assert_eq!(
            message.downcast_ref::<String>().map(String::as_str),
            Some("note:delete:2")
        );
    }

    #[test]
    fn render_applies_one_batch() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let mut renderer = RecordingRenderer::default();

        runtime.render(&screen("Hello"), viewport(), &mut renderer);
        runtime.render(&screen("Hello"), viewport(), &mut renderer);

        assert_eq!(renderer.batches.len(), 2);
        assert!(!renderer.batches[0].is_empty());
        assert!(renderer.batches[1].is_empty());
    }

    #[test]
    fn a_narrower_viewport_moves_existing_widgets() {
        struct TwoLines;

        impl Measure for TwoLines {
            fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
                let width = constraints.max_width.unwrap_or(200.0);
                let lines = if text.content.len() > 10 { 2.0 } else { 1.0 };
                Size::new(width, 20.0 * lines)
            }
        }

        let element: Element = Column::new()
            .width_percent(1.0)
            .child(Text::new("a long text"))
            .into();

        let mut runtime = Runtime::new(Box::new(TwoLines));
        runtime.update(&element, Size::new(300.0, 600.0));
        let update = runtime.update(&element, Size::new(100.0, 600.0));

        assert!(
            update.ops.iter().any(|op| matches!(op, Op::Layout { .. })),
            "{:?}",
            update.ops
        );
        assert!(
            !update
                .ops
                .iter()
                .any(|op| matches!(op, Op::Create(_) | Op::Update(_))),
            "{:?}",
            update.ops
        );
    }

    fn list_element(count: usize) -> Element {
        List::new(Arc::new(
            (0..count)
                .map(|index| format!("Row {index}"))
                .collect::<Vec<_>>(),
        ))
        .item_height(50.0)
        .buffer(2)
        .key(|item| item.clone())
        .row(|item| Text::new(item.clone()).into())
        .into()
    }

    #[test]
    fn scrolling_within_a_row_does_not_rebuild() {
        let element = list_element(100);
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(300.0, 200.0));
        let list_id = runtime.tree().children[0].id;

        // The first row is 50 points tall, so 10 points stay in the same window.
        assert!(!runtime.scroll(list_id, 10.0));
        assert_eq!(runtime.scroll_offset(list_id), 10.0);
    }

    #[test]
    fn scrolling_past_a_row_asks_for_a_rebuild() {
        let element = list_element(100);
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(300.0, 200.0));
        let list_id = runtime.tree().children[0].id;

        assert!(runtime.scroll(list_id, 60.0));
    }

    #[test]
    fn scrolling_is_clamped_to_the_content() {
        let element = list_element(10);
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(300.0, 200.0));
        let list_id = runtime.tree().children[0].id;

        // 10 rows of 50 points in a 200-point viewport: 300 points of travel.
        runtime.scroll(list_id, 10_000.0);
        assert_eq!(runtime.scroll_offset(list_id), 300.0);
    }

    #[test]
    fn a_rebuild_uses_the_recorded_scroll_offset() {
        let element = list_element(100);
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(300.0, 200.0));
        let list_id = runtime.tree().children[0].id;

        assert!(runtime.scroll(list_id, 350.0));
        runtime.update(&element, Size::new(300.0, 200.0));

        let scrolled = &runtime.tree().children[0].children[0].children;
        // A leading spacer marks the five rows that scrolled above the window
        // (row 7 is the first visible one, with a buffer of two).
        assert!(matches!(scrolled[0].kind, NodeKind::Spacer));
        assert_eq!(scrolled[0].style.height, Length::Points(5.0 * 50.0));
        assert_eq!(
            scrolled.last().unwrap().style.height,
            Length::Points((100.0 - 13.0) * 50.0)
        );
    }

    #[test]
    fn variable_rows_are_measured_and_scrollable() {
        let element: Element = List::new(Arc::new(
            (0..200)
                .map(|index| "word ".repeat(1 + index % 5))
                .collect::<Vec<_>>(),
        ))
        .buffer(2)
        .key(|item| item.clone())
        .row(|item| Text::new(item.clone()).into())
        .into();

        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(120.0, 200.0));
        let list_id = runtime.tree().children[0].id;

        let measurement = &runtime.measurements[&list_id];
        assert_eq!(measurement.prefix.len(), 201);
        assert!(measurement.heights.len() >= 4);
        let mut distinct: Vec<i32> = measurement
            .heights
            .values()
            .map(|height| (height * 10.0).round() as i32)
            .collect();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.len() > 1, "rows should have different heights");

        // Repeating an offset changes nothing.
        assert!(!runtime.scroll(list_id, 0.0));
        // Far enough to move the materialized range.
        assert!(runtime.scroll(list_id, 600.0));

        runtime.update(&element, Size::new(120.0, 200.0));
        let scrolled = &runtime.tree().children[0].children[0].children;
        assert!(matches!(scrolled[0].kind, NodeKind::Spacer));
    }

    #[test]
    fn a_focused_row_stays_materialized_while_scrolling() {
        let element: Element = List::new(Arc::new(
            (0..100)
                .map(|index| format!("Row {index}"))
                .collect::<Vec<_>>(),
        ))
        .item_height(50.0)
        .buffer(3)
        .key(|item| item.clone())
        .row(|item| Text::new(item.clone()).into())
        .into();

        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(300.0, 200.0));
        let list_id = runtime.tree().children[0].id;
        let row = &runtime.tree().children[0].children[0].children[2];
        let focused = row.children[0].id;

        assert!(runtime.set_focus(focused, true));
        runtime.update(&element, Size::new(300.0, 200.0));
        assert_eq!(runtime.focused_rows[&list_id], 2);

        // Far past the focused row: the window still has to include it.
        assert!(runtime.scroll(list_id, 900.0));
        runtime.update(&element, Size::new(300.0, 200.0));
        assert!(tree::find(runtime.tree(), focused).is_some());
        assert_eq!(runtime.focused_rows[&list_id], 2);

        // Releasing focus lets the row leave the window again.
        assert!(runtime.set_focus(focused, false));
        runtime.update(&element, Size::new(300.0, 200.0));
        assert!(runtime.focused_rows.is_empty());
        assert!(tree::find(runtime.tree(), focused).is_none());
    }

    #[test]
    fn row_focus_survives_recycling_until_the_user_blurs() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());

        let list = NodeId(7);
        let field = NodeId(70);
        assert!(runtime.set_row_focus(list, 3, field, true));
        assert_eq!(runtime.row_focus().unwrap().index, 3);

        // A blur that follows the row being torn down is ignored.
        assert!(runtime.park_row_focus(field));
        assert!(!runtime.set_row_focus(list, 3, field, false));
        assert!(runtime.row_focus().is_some());

        // A real blur while the row is mounted clears the logical focus.
        assert!(runtime.set_row_focus(list, 3, field, true));
        assert!(runtime.set_row_focus(list, 3, field, false));
        assert!(runtime.row_focus().is_none());

        // A main-tree input takes over the focus.
        runtime.set_row_focus(list, 3, field, true);
        runtime.set_focus(NodeId(1), true);
        assert!(runtime.row_focus().is_none());
    }

    #[test]
    fn a_viewport_change_suppresses_layout_animations() {
        let element: Element = Text::new("animated")
            .height_percent(0.5)
            .animate(300)
            .into();

        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(200.0, 500.0));
        let update = runtime.update(&element, Size::new(200.0, 400.0));

        let animations: Vec<_> = update
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Layout { animation, .. } => Some(animation.clone()),
                _ => None,
            })
            .collect();
        assert!(!animations.is_empty(), "{:?}", update.ops);
        assert!(animations.iter().all(Option::is_none), "{animations:?}");
    }

    #[test]
    fn a_different_list_on_the_same_node_resets_the_scroll() {
        let element = |prefix: &str| -> Element {
            List::new(Arc::new(
                (0..100)
                    .map(|index| format!("{prefix}{index}"))
                    .collect::<Vec<_>>(),
            ))
            .buffer(2)
            .key(|item| item.clone())
            .row(|item| Text::new(item.clone()).into())
            .into()
        };

        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element("A"), Size::new(200.0, 200.0));
        let id = runtime.tree().children[0].id;

        assert!(runtime.scroll(id, 500.0));
        runtime.update(&element("A"), Size::new(200.0, 200.0));
        assert!(runtime.scroll_offset(id) > 0.0);

        // Same position, so the same node id, but a different list: the
        // previous scroll position and heights must not leak into it.
        runtime.update(&element("B"), Size::new(200.0, 200.0));
        assert_eq!(runtime.scroll_offset(id), 0.0);
        assert_eq!(runtime.measurements[&id].window.0, 0);
    }

    #[test]
    fn scrolling_an_unknown_node_is_ignored() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());

        assert!(!runtime.scroll(NodeId(999), 100.0));
    }

    #[test]
    fn a_measured_height_patches_the_cached_prefix() {
        let count = 100;
        let element: Element = List::new(Arc::new((0..count).collect::<Vec<usize>>()))
            .revision(1)
            .buffer(0)
            .key(|index| format!("Row {index}"))
            .row(|index| Text::new(format!("Row {index}")).into())
            .into();

        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&element, Size::new(300.0, 2_000.0));
        let id = runtime.tree().children[0].id;

        let index = 5;
        let key_hash = tree::hash_string(&format!("Row {index}"));
        let measurement = &runtime.measurements[&id];
        let previous = measurement
            .heights
            .get(&key_hash)
            .copied()
            .unwrap_or(measurement.estimate);
        let before = measurement.prefix.clone();

        let row_id = NodeId(999_999);
        let info = BuildInfo {
            rows: HashMap::from([(id, vec![(row_id, key_hash, index)])]),
            ..BuildInfo::default()
        };
        let mut layout = LayoutResult::default();
        layout.frames.insert(
            row_id,
            crate::geometry::Rect::new(0.0, 0.0, 300.0, previous + 10.0),
        );

        assert!(runtime.record_heights(&info, &layout));

        let measurement = &runtime.measurements[&id];
        assert_eq!(measurement.heights[&key_hash], previous + 10.0);
        assert_eq!(measurement.prefix[index], before[index]);
        for (item, previous) in before.iter().enumerate().take(count + 1).skip(index + 1) {
            assert_eq!(measurement.prefix[item], previous + 10.0);
        }
    }
}
