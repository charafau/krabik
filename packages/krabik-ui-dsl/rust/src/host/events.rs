//! Turning a native event into an application message.
//!
//! Every platform reports "widget `id` did X". This module finds the handler
//! the application attached for that and produces its message.

use crate::engine::tree::{NodeKind, UiNode};
use crate::message::{Handler, Message};
use crate::ui::{DatePickerValue, DateValue, TimeValue, snap_slider_value};
use crate::widgets::Widget;

/// A native event on one widget, before it is matched to an action.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum NodeEvent {
    /// A button was pressed.
    Click,
    /// A container was pressed and held.
    LongPress,
    /// The text of an input changed.
    Text(String),
    /// An input was submitted from the keyboard.
    Submit,
    /// An input gained or lost focus. `attached` is false when the field
    /// lost focus because its recycled row left the screen.
    Focus { focused: bool, attached: bool },
    /// A switch, checkbox or radio button was toggled.
    Toggle(bool),
    /// A slider moved to a raw, unsnapped value.
    Slider(f32),
    /// A thumb of a range slider moved; both places along the track, each
    /// from 0 to 1.
    Range(f32, f32),
    /// A radio group, tab bar or pager selected an index.
    Select(usize),
    /// A picker chose a value, before clamping to the widget's range.
    Picker(DatePickerValue),
    /// A view of the application's own sent a string.
    Native(String),
    /// A scroll container moved to an offset in logical points.
    Scroll(f32),
    /// A scroll container or a list was pulled down to refresh.
    Refresh,
    /// A row of a list was swiped away.
    Dismiss(usize),
    /// A row of a list was dragged from one index to another.
    Move(usize, usize),
}

/// What the application wants to hear about from one widget.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Actions {
    pub click: Handler<()>,
    pub long_press: Handler<()>,
    pub text: Handler<String>,
    pub native: Handler<String>,
    pub submit: Handler<()>,
    pub focus: Handler<bool>,
    pub toggle: Handler<bool>,
    pub select: Handler<usize>,
    /// The number of choices, where an index past the end must be dropped:
    /// a menu that was open while the application replaced it.
    pub select_count: Option<usize>,
    pub refresh: Handler<()>,
    pub dismiss: Handler<usize>,
    pub reorder: Handler<(usize, usize)>,
    /// The number of rows of a list, past which an index is dropped.
    pub row_count: usize,
    pub slider: Option<SliderAction>,
    pub range: Option<crate::ui::RangeSliderProps>,
    pub picker: Option<PickerAction>,
}

/// A slider's handler and the range its values snap to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SliderAction {
    pub on_change: Handler<f32>,
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
}

/// A picker's handler and the range its values are clamped to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PickerAction {
    pub on_change: Handler<DatePickerValue>,
    pub date_range: Option<(DateValue, DateValue)>,
    pub time_range: Option<(TimeValue, TimeValue)>,
}

impl Actions {
    /// The handlers a node carries, or `None` when it carries none.
    pub(crate) fn of(node: &UiNode) -> Option<Self> {
        let mut actions = Self::default();
        match &node.kind {
            NodeKind::Widget(widget) => match widget {
                Widget::Button(props) => actions.click = props.on_click.clone(),
                Widget::Switch(props) => actions.toggle = props.on_toggle.clone(),
                Widget::Checkbox(props) | Widget::RadioButton(props) => {
                    actions.toggle = props.on_toggle.clone();
                }
                Widget::RadioGroup(props) => actions.select = props.on_select.clone(),
                Widget::TabBar(props) => actions.select = props.on_select.clone(),
                Widget::DatePicker(props) => {
                    actions.picker = props.on_change.is_set().then(|| PickerAction {
                        on_change: props.on_change.clone(),
                        date_range: props.date_range,
                        time_range: props.time_range,
                    });
                }
                Widget::Slider(props) => {
                    actions.slider = props.on_change.is_set().then(|| SliderAction {
                        on_change: props.on_change.clone(),
                        min: props.min,
                        max: props.max,
                        step: props.step,
                    });
                }
                Widget::RangeSlider(props) => {
                    actions.range = props.on_change.is_set().then(|| props.clone());
                }
                Widget::NativeView(props) => actions.native = props.on_event.clone(),
                Widget::TextField(props) => {
                    actions.text = props.on_change.clone();
                    actions.submit = props.on_submit.clone();
                    actions.focus = props.on_focus.clone();
                }
                Widget::Text(_) | Widget::Progress(_) | Widget::Spinner(_) | Widget::Image(_) => {}
            },
            NodeKind::Pager(props) => actions.select = props.on_select.clone(),
            NodeKind::Container(props) => {
                actions.click = props.on_tap.clone();
                actions.long_press = props.on_long_press.clone();
                if let Some(menu) = &props.menu {
                    actions.select = menu.on_select.clone();
                    actions.select_count = Some(menu.items.len());
                }
            }
            NodeKind::Scroll(props) => actions.refresh = props.refresh.on_refresh.clone(),
            NodeKind::Recycler(props) => {
                actions.refresh = props.refresh.on_refresh.clone();
                actions.dismiss = props.on_dismiss.clone();
                actions.reorder = props.on_reorder.clone();
                if actions.dismiss.is_set() || actions.reorder.is_set() {
                    actions.row_count = props.item_count;
                }
            }
            NodeKind::Root
            | NodeKind::Column
            | NodeKind::Row
            | NodeKind::Spacer
            | NodeKind::Stack => {}
        }
        (actions != Self::default()).then_some(actions)
    }

    /// The message a native event stands for, if the widget reports it.
    pub(crate) fn message(&self, event: &NodeEvent) -> Option<Message> {
        match event {
            NodeEvent::Click => self.click.message(()),
            NodeEvent::LongPress => self.long_press.message(()),
            NodeEvent::Text(value) => self.text.message(value.clone()),
            NodeEvent::Submit => self.submit.message(()),
            NodeEvent::Native(value) => self.native.message(value.clone()),
            NodeEvent::Focus { focused, .. } => self.focus.message(*focused),
            NodeEvent::Toggle(checked) => self.toggle.message(*checked),
            NodeEvent::Select(index) => {
                if self.select_count.is_some_and(|count| *index >= count) {
                    return None;
                }
                self.select.message(*index)
            }
            NodeEvent::Refresh => self.refresh.message(()),
            // The list may have changed while the row was under the finger.
            NodeEvent::Dismiss(index) if *index < self.row_count => self.dismiss.message(*index),
            NodeEvent::Move(from, to) if *from < self.row_count && *to < self.row_count => {
                self.reorder.message((*from, *to))
            }
            NodeEvent::Dismiss(_) | NodeEvent::Move(..) => None,
            NodeEvent::Slider(value) => self.slider.as_ref().and_then(|slider| {
                let value = snap_slider_value(*value, slider.min, slider.max, slider.step);
                slider.on_change.message(value)
            }),
            NodeEvent::Range(low, high) => self
                .range
                .as_ref()
                .and_then(|range| range.on_change.message(range.values_at(*low, *high))),
            NodeEvent::Picker(value) => self.picker.as_ref().and_then(|picker| {
                let value = value.clamped(picker.date_range, picker.time_range);
                picker.on_change.message(value)
            }),
            // Scrolling is engine state, not an application event.
            NodeEvent::Scroll(_) => None,
        }
    }
}
