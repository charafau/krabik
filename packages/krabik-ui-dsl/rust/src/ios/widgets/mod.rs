//! Per-widget updates. [`update_content`] applies a node to the view that was created for it.

use crate::ios::host::Content;
use crate::ios::widgets::button::update_button;
use crate::ios::widgets::check::update_check;
use crate::ios::widgets::date_picker::update_date_picker;
use crate::ios::widgets::progress::{update_progress, update_spinner};
use crate::ios::widgets::radio_group::update_radio_group;
use crate::ios::widgets::slider::update_slider;
use crate::ios::widgets::tab_bar::update_tab_bar;
use crate::ios::widgets::text::update_label;
use crate::ios::widgets::text_field::update_field;
use crate::ios::{IosState, NodeId, ViewHandle};
use crate::render::RenderNode;
use crate::tree::NodeKind;
use crate::widgets::Widget;
use objc2::MainThreadMarker;
use objc2_core_foundation::CGPoint;

pub(crate) mod button;
pub(crate) mod check;
pub(crate) mod date_picker;
pub(crate) mod pager;
pub(crate) mod progress;
pub(crate) mod radio_group;
pub(crate) mod scroll;
pub(crate) mod slider;
pub(crate) mod switch;
pub(crate) mod tab_bar;
pub(crate) mod text;
pub(crate) mod text_field;

/// Applies a node's own properties to the view that was created for it.
///
/// The match on the node kind is exhaustive, so a new widget cannot be
/// created and then silently ignore later changes. Each arm expects the
/// content its creation stored; a mismatch means the node changed kind,
/// which the engine handles by recreating the view.
pub(crate) fn update_content(
    content: &mut Content,
    node: &RenderNode,
    handle: u64,
    host: u64,
    mtm: MainThreadMarker,
) {
    match &node.kind {
        NodeKind::Widget(widget) => match (widget, content) {
            (Widget::Text(props), Content::Label(label)) => update_label(label, props),
            (Widget::TextField(props), Content::Field(field)) => update_field(field, props),
            (Widget::Button(props), Content::Button(button)) => update_button(button, props),
            (Widget::Switch(props), Content::Switch(switch)) => switch.setOn(props.checked),
            (
                Widget::Checkbox(props) | Widget::RadioButton(props),
                Content::Check { button, radio },
            ) => update_check(button, props, *radio),
            (
                Widget::RadioGroup(props),
                Content::RadioGroup {
                    stack,
                    buttons,
                    labels,
                },
            ) => update_radio_group(stack, buttons, labels, props, handle, host, node.id, mtm),
            (Widget::DatePicker(props), Content::DatePicker(picker)) => {
                update_date_picker(picker, props);
            }
            (Widget::TabBar(props), Content::TabBar(control)) => update_tab_bar(control, props),
            (Widget::Slider(props), Content::Slider(slider)) => update_slider(slider, props),
            (Widget::Progress(props), Content::Progress(progress)) => {
                update_progress(progress, props);
            }
            (Widget::Spinner(props), Content::Spinner(spinner)) => update_spinner(spinner, props),
            // Images are updated by `IosState::load_image`, which starts
            // downloads and therefore needs the state itself.
            (Widget::Image(_), _) => {}
            // The stored content does not belong to this widget.
            (
                Widget::Text(_)
                | Widget::TextField(_)
                | Widget::Button(_)
                | Widget::Switch(_)
                | Widget::Checkbox(_)
                | Widget::RadioButton(_)
                | Widget::RadioGroup(_)
                | Widget::DatePicker(_)
                | Widget::TabBar(_)
                | Widget::Slider(_)
                | Widget::Progress(_)
                | Widget::Spinner(_),
                _,
            ) => {}
        },
        NodeKind::Pager(props) => {
            if let Content::Pager {
                scroll,
                pages,
                selected,
                ..
            } = content
            {
                *pages = props.pages;
                let target = if props.pages == 0 {
                    0
                } else {
                    props.selected.min(props.pages - 1)
                };
                if *selected != target {
                    *selected = target;
                    let width = scroll.bounds().size.width;
                    if width > 0.0 {
                        scroll.setContentOffset_animated(
                            CGPoint::new(width * target as f64, 0.0),
                            true,
                        );
                    }
                }
            }
        }
        // Containers have no properties of their own; a recycler is updated
        // through its data source by `IosState::apply_to`.
        NodeKind::Root
        | NodeKind::Column
        | NodeKind::Row
        | NodeKind::Spacer
        | NodeKind::Scroll(_)
        | NodeKind::Recycler(_) => {}
    }
}

/// Where a new native view is being created, and where its events go.
pub(crate) struct Site {
    /// The session the view belongs to.
    pub(crate) handle: u64,
    /// The host inside that session.
    pub(crate) host: u64,
    /// The node the view is created for.
    pub(crate) node: NodeId,
    /// Proof that the view is created on the main thread.
    pub(crate) mtm: MainThreadMarker,
}

impl IosState {
    /// Creates the native view of a widget, configured and wired to its
    /// events.
    pub(crate) fn create_widget(
        &mut self,
        widget: &Widget,
        node: &RenderNode,
        site: &Site,
    ) -> ViewHandle {
        match widget {
            Widget::Text(props) => text::create(props, node, site),
            Widget::Button(props) => button::create(props, node, site),
            Widget::TextField(props) => text_field::create(props, node, site),
            Widget::Switch(props) => switch::create(props, node, site),
            Widget::Checkbox(props) => check::create(props, node, site, false),
            Widget::RadioButton(props) => check::create(props, node, site, true),
            Widget::RadioGroup(props) => radio_group::create(props, node, site),
            Widget::TabBar(props) => tab_bar::create(props, node, site),
            Widget::DatePicker(props) => date_picker::create(props, node, site),
            Widget::Slider(props) => slider::create(props, node, site),
            Widget::Progress(props) => progress::create(props, node, site),
            Widget::Spinner(props) => progress::create_spinner(props, node, site),
            Widget::Image(props) => self.create_image(props, node, site.mtm),
        }
    }
}
