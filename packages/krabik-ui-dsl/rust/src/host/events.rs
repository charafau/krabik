//! Turning a native event into an application [`Event`].
//!
//! Every platform reports "widget `id` did X". This module finds what the
//! application asked to be told about that widget and builds the event.

use std::borrow::Cow;

use crate::app::Event;
use crate::engine::tree::{NodeKind, UiNode};
use crate::ui::{DatePickerValue, DateValue, TimeValue, snap_slider_value};
use crate::widgets::Widget;

/// A native event on one widget, before it is matched to an action.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum NodeEvent {
    /// A button was pressed.
    Click,
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
    /// A radio group, tab bar or pager selected an index.
    Select(usize),
    /// A picker chose a value, before clamping to the widget's range.
    Picker(DatePickerValue),
    /// A scroll container moved to an offset in logical points.
    Scroll(f32),
}

/// What the application wants to hear about from one widget.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Actions {
    pub click: Option<Cow<'static, str>>,
    pub text: Option<Cow<'static, str>>,
    pub submit: Option<Cow<'static, str>>,
    pub focus: Option<Cow<'static, str>>,
    pub toggle: Option<Cow<'static, str>>,
    pub select: Option<Cow<'static, str>>,
    pub slider: Option<SliderAction>,
    pub picker: Option<PickerAction>,
}

/// A slider's action and the range its values snap to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SliderAction {
    pub action: Cow<'static, str>,
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
}

/// A picker's action and the range its values are clamped to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PickerAction {
    pub action: Cow<'static, str>,
    pub date_range: Option<(DateValue, DateValue)>,
    pub time_range: Option<(TimeValue, TimeValue)>,
}

/// An action, unless the widget left it empty.
fn some(action: &str) -> Option<Cow<'static, str>> {
    (!action.is_empty()).then(|| Cow::Owned(action.to_owned()))
}

impl Actions {
    /// The actions a node declares, or `None` when it declares none.
    pub(crate) fn of(node: &UiNode) -> Option<Self> {
        let mut actions = Self::default();
        match &node.kind {
            NodeKind::Widget(widget) => match widget {
                Widget::Button(props) => actions.click = some(&props.action),
                Widget::Switch(props) => actions.toggle = some(&props.action),
                Widget::Checkbox(props) | Widget::RadioButton(props) => {
                    actions.toggle = some(&props.action);
                }
                Widget::RadioGroup(props) => actions.select = some(&props.action),
                Widget::TabBar(props) => actions.select = some(&props.action),
                Widget::DatePicker(props) => {
                    actions.picker = some(&props.action).map(|action| PickerAction {
                        action,
                        date_range: props.date_range,
                        time_range: props.time_range,
                    });
                }
                Widget::Slider(props) => {
                    actions.slider = some(&props.action).map(|action| SliderAction {
                        action,
                        min: props.min,
                        max: props.max,
                        step: props.step,
                    });
                }
                Widget::TextField(props) => {
                    actions.text = some(&props.action);
                    actions.submit = some(&props.submit_action);
                    actions.focus = some(&props.focus_action);
                }
                Widget::Text(_) | Widget::Progress(_) | Widget::Spinner(_) | Widget::Image(_) => {}
            },
            NodeKind::Pager(props) => actions.select = some(&props.action),
            NodeKind::Root
            | NodeKind::Column
            | NodeKind::Row
            | NodeKind::Spacer
            | NodeKind::Scroll(_)
            | NodeKind::Recycler(_) => {}
        }
        (actions != Self::default()).then_some(actions)
    }

    /// Hands the application the event a native event stands for, if the
    /// widget asked for it. Returns whether the application changed.
    pub(crate) fn deliver(
        &self,
        event: &NodeEvent,
        mut handle: impl FnMut(Event<'_>) -> bool,
    ) -> bool {
        match event {
            NodeEvent::Click => self
                .click
                .as_deref()
                .is_some_and(|action| handle(Event::Click { action })),
            NodeEvent::Text(value) => self
                .text
                .as_deref()
                .is_some_and(|action| handle(Event::TextChanged { action, value })),
            NodeEvent::Submit => self
                .submit
                .as_deref()
                .is_some_and(|action| handle(Event::Submitted { action })),
            NodeEvent::Focus { focused, .. } => self.focus.as_deref().is_some_and(|action| {
                handle(Event::FocusChanged {
                    action,
                    focused: *focused,
                })
            }),
            NodeEvent::Toggle(checked) => self.toggle.as_deref().is_some_and(|action| {
                handle(Event::Toggled {
                    action,
                    checked: *checked,
                })
            }),
            NodeEvent::Select(index) => self.select.as_deref().is_some_and(|action| {
                handle(Event::Selected {
                    action,
                    index: *index,
                })
            }),
            NodeEvent::Slider(value) => self.slider.as_ref().is_some_and(|slider| {
                handle(Event::ValueChanged {
                    action: &slider.action,
                    value: snap_slider_value(*value, slider.min, slider.max, slider.step),
                })
            }),
            NodeEvent::Picker(value) => self.picker.as_ref().is_some_and(|picker| {
                let action = &*picker.action;
                match value.clamped(picker.date_range, picker.time_range) {
                    DatePickerValue::Date(date) => handle(Event::DateChanged { action, date }),
                    DatePickerValue::Time(time) => handle(Event::TimeChanged { action, time }),
                    DatePickerValue::DateTime { date, time } => {
                        handle(Event::DateTimeChanged { action, date, time })
                    }
                }
            }),
            // Scrolling is engine state, not an application event.
            NodeEvent::Scroll(_) => false,
        }
    }
}
